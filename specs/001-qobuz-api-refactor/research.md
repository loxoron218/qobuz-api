# Research: Qobuz API Client Refactor

**Feature**: `001-qobuz-api-refactor` | **Date**: 2026-04-06

## 1. HTTP Client Architecture

### Decision: Single `reqwest::Client` with streaming downloads

**Rationale**: The constitution mandates a single `reqwest::Client` instance with connection pooling (Principle IV). `reqwest` handles connection pooling by default — each `Client` maintains an internal connection pool. No per-request client construction is allowed. The `stream` feature enables `bytes_stream()` for chunked downloads without loading entire files into memory.

**Alternatives considered**:
- `reqwest-middleware` with retry layers: Adds complexity; retry logic can be implemented with a simple wrapper function.
- `hyper` directly: Too low-level; `reqwest` provides the right abstraction.

**Implementation pattern**:
```rust
let client = Client::builder()
    .user_agent("qobuz-api")
    .default_headers(default_headers)
    .build()?;
```
Stream downloads via `response.bytes_stream()` with `tokio::io::BufWriter` for efficient disk I/O.

---

## 2. Concurrent Download Model

### Decision: `tokio::sync::Semaphore` for bounded concurrency

**Rationale**: The spec requires configurable concurrent downloads (default: 4). `tokio::sync::Semaphore` with `acquire_owned()` provides the canonical Rust pattern for limiting concurrent async tasks. Each download task acquires a permit, performs the download, and drops the permit on completion. This is non-blocking and integrates naturally with the Tokio runtime.

**Alternatives considered**:
- `futures::stream::buffered()`: Less explicit control over concurrency semantics.
- `async-channel` bounded channel: Over-engineered for simple concurrency limiting.
- Custom thread pool: Unnecessary since downloads are I/O-bound, not CPU-bound.

---

## 3. Metadata Embedding

### Decision: `lofty` crate with format-specific tag writing

**Rationale**: `lofty` provides a unified `Tag` trait that abstracts over FLAC (Vorbis Comments) and MP3 (ID3v2) formats. It supports `set_title()`, `set_artist()`, `set_album()`, custom `ItemKey` entries, and `Picture` embedding for cover art. The original codebase already uses `lofty` 0.22.x; the refactor continues with the latest version.

**Implementation pattern**:
```rust
let mut tagged_file = Probe::open(&path)?.read()?;
let tag = tagged_file.primary_tag_mut().unwrap();
tag.set_title(track_title);
tag.push_picture(cover_art);
tag.save_to_path(&path, WriteOptions::default())?;
```

**Key considerations**:
- FLAC uses Vorbis Comments (string key-value pairs); `ItemKey` maps to standard keys.
- MP3 uses ID3v2 frames; `lofty` handles encoding automatically.
- Cover art: `Picture::new_unchecked(PictureType::CoverFront, MimeType::Jpeg, ...)` for front cover.
- CPU-bound metadata tagging should use `rayon` for batch operations (per constitution Principle IV).

---

## 4. Request Signing

### Decision: MD5-based signature generation matching Qobuz API protocol

**Rationale**: The Qobuz API requires MD5-based request signatures for authenticated endpoints (favorites, file URLs). The signing algorithm is:
1. General signed GET: Sort params alphabetically, concatenate `METHOD + endpoint + key1value1... + app_secret`, MD5 hash.
2. Track file URL: Build fixed-format string `"trackgetFileUrlformat_id{fid}intentstreamtrack_id{tid}{ts}{secret}"`, MD5 hash.

The `md5` crate provides the necessary hashing. The original codebase's algorithm is preserved exactly.

**Alternatives considered**:
- Using a different hash crate: `md5` is the lightest option and matches the API protocol.
- Moving to a more modern hash: Not possible — the API mandates MD5.

---

## 5. Credential Management

### Decision: `.env` file with `dotenvy` + web player JS extraction

**Rationale**: The spec requires credentials stored in `.env` files with `0600` permissions. `dotenvy` is the maintained fork of `dotenv` for Rust. App credentials (app_id, app_secret) are extracted from the Qobuz web player's JavaScript bundle using `regex` patterns, then cached in `.env`. User credentials come from env vars (`QOBUZ_USER_ID`, `QOBUZ_USER_AUTH_TOKEN`, `QOBUZ_EMAIL`, `QOBUZ_PASSWORD`).

**File permissions**: Use `std::fs::set_permissions` with `Permissions::from_mode(0o600)` on Linux to enforce owner-only access.

**Web player extraction flow**:
1. Fetch `https://play.qobuz.com/login`
2. Extract bundle.js URL via regex
3. Extract `appId` and `initialSeed` from bundle.js
4. Decode app_secret from base64

---

## 6. Error Handling Strategy

### Decision: `thiserror` for library errors, `anyhow` for binary

**Rationale**: The constitution mandates `thiserror` for library crate error types and `anyhow` at the binary top-level only. All errors must be `Send + Sync + 'static` for async Tokio tasks. The error enum covers: authentication, HTTP/network, API response parsing, metadata, download, rate limiting, resource not found, and invalid parameters.

**Error type structure**:
```rust
#[derive(Error, Debug)]
pub enum QobuzApiError {
    #[error("Authentication failed: {message}")]
    AuthenticationError { message: String },
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    // ... other variants
}
```

**Rate limiting**: Automatic retry with exponential backoff (max 3 retries). Implementation uses `tokio::time::sleep` with `Duration::from_millis(base * 2^attempt)`.

---

## 7. Module Organization

### Decision: Capability-grouped modules, no `models/handlers/utils` anti-pattern

**Rationale**: The constitution explicitly forbids the `models/handlers/utils` structure. Modules are grouped by domain:
- `api/` — HTTP client layer (service, requests, auth, content submodules, favorites)
- `models/` — Pure data structures for API responses
- `metadata/` — Audio file metadata extraction and embedding
- Top-level: `errors.rs`, `signing.rs`, `credentials.rs`, `sanitize.rs`

Each file stays under 400 lines. Split impl blocks across files within the same module are acceptable.

---

## 8. Testing Strategy

### Decision: Unit tests at file bottom + integration tests with deterministic mocks

**Rationale**: The constitution mandates Red-Green-Refactor. Unit tests live at the bottom of each source file in `#[cfg(test)] mod tests`. Integration tests use mocked HTTP responses (via a custom test harness intercepting `reqwest` calls or a mock server). `tempfile` for filesystem fixtures. No real network calls in CI.

**Alternatives considered**:
- `wiremock-rs`: Good for HTTP mocking but adds a dependency; simpler approach is a trait-based client abstraction.
- `mockito`: Similar to wiremock; trait abstraction is more idiomatic Rust.

**Recommended approach**: Define a `HttpClient` trait with a `reqwest` implementation and a mock implementation. This allows deterministic testing without external dependencies.

---

## 9. Performance Considerations

### Decision: `tokio` for I/O, `rayon` for CPU-bound metadata tagging

**Rationale**: The constitution mandates:
- `tokio` tasks for I/O-bound concurrency (HTTP requests, file downloads)
- `rayon` for CPU-bound parallelism (metadata tagging of batch downloads)
- Credential refresh at most once per session (not per-track)
- `criterion` benchmarks for hot paths

**Hot paths to benchmark**:
- Album download pipeline (concurrent track downloads + metadata embedding)
- Metadata embedding (tag writing for FLAC and MP3)
- Search result deserialization (serde parsing)
- Request signature generation (MD5 hashing)
