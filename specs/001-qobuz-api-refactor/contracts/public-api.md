# Public API Contract: Qobuz API Rust Refactor

**Feature**: `001-qobuz-api-refactor` | **Date**: 2026-04-06

This document defines the public API surface of the `qobuz-api` library crate.

---

## Library Re-exports (`lib.rs`)

```rust
pub use api::http_client::HttpClient;
pub use api::service::QobuzApiService;
pub use errors::QobuzApiError;
pub use metadata::config::MetadataConfig;
pub use metadata::embedder::embed_metadata_in_file;
pub use metadata::extractor::extract_comprehensive_metadata;
pub use models::album::Album;
pub use models::artist::Artist;
pub use models::track::Track;
pub use models::playlist::Playlist;
pub use models::search::{ItemSearchResult, SearchResult};
pub use models::favorites::UserFavorites;
pub use models::file_url::FileUrl;
pub use models::credential::Credential;
pub use sanitize::sanitize_filename;
```

---

## HttpClient Trait

### `HttpClient` (trait, `src/api/http_client.rs`)

Trait abstraction over HTTP operations enabling deterministic testing via a mock implementation. Production code uses `ReqwestClient` (wrapping `reqwest::Client`); tests use `MockHttpClient`.

```rust
pub trait HttpClient: Send + Sync {
    async fn get(&self, url: &str, headers: HeaderMap) -> Result<Response, QobuzApiError>;
    async fn post(&self, url: &str, headers: HeaderMap, body: Option<String>) -> Result<Response, QobuzApiError>;
    async fn signed_get(&self, url: &str, params: &[(&str, &str)], app_secret: &str) -> Result<Response, QobuzApiError>;
    async fn bytes_stream(&self, url: &str) -> Result<impl Stream<Item = Result<Bytes, QobuzApiError>>, QobuzApiError>;
}
```

**Design rationale**: Per research.md section 8, trait abstraction is the recommended approach for deterministic integration testing without real network calls.

---

## QobuzApiService

### Constructors

#### `QobuzApiService::new() -> Result<Self, QobuzApiError>`

Creates a new service instance. Extracts app credentials from `.env` or the Qobuz web player JS bundle.

**Guarantees**: Returns a service with valid `app_id` and `app_secret`. HTTP client is initialized with connection pooling. `user_auth_token` is `None` (unauthenticated state).

#### `QobuzApiService::with_credentials(app_id: &str, app_secret: &str) -> Result<Self, QobuzApiError>`

Creates a service with explicitly provided app credentials.

### Authentication

#### `authenticate_with_env(&mut self) -> Result<(), QobuzApiError>`

Reads credentials from environment variables in priority order:
1. `QOBUZ_USER_ID` + `QOBUZ_USER_AUTH_TOKEN` → token-based auth
2. `QOBUZ_EMAIL` + `QOBUZ_PASSWORD` → email/password auth
3. `QOBUZ_USERNAME` + `QOBUZ_PASSWORD` → username/password auth

**Post-condition**: `user_auth_token` is `Some`.

#### `login(&mut self, email: &str, password: &str) -> Result<(), QobuzApiError>`

Authenticates with email and MD5-hashed password via `POST /user/login`.

**Pre-condition**: None (works without existing auth).
**Post-condition**: `user_auth_token` is `Some`.

#### `login_with_token(&mut self, user_id: &str, auth_token: &str) -> Result<(), QobuzApiError>`

Authenticates with user ID and auth token via `POST /user/login`.

**Post-condition**: `user_auth_token` is `Some`.

#### `refresh_app_credentials(&mut self) -> Result<(), QobuzApiError>`

Re-extracts app credentials from the Qobuz web player. Writes updated credentials to `.env`.

**Pre-condition**: Can only be called once per session. Returns error on subsequent calls.
**Post-condition**: `credentials_refreshed` is `true`.

### Search

#### `search_catalog(&self, query: &str, limit: Option<i32>, offset: Option<i32>) -> Result<SearchResult, QobuzApiError>`

Searches all content types. Returns grouped results.

#### `search_albums(&self, query: &str, limit: Option<i32>, offset: Option<i32>) -> Result<ItemSearchResult<Box<Album>>, QobuzApiError>`

Searches albums only.

#### `search_artists(&self, query: &str, limit: Option<i32>, offset: Option<i32>) -> Result<ItemSearchResult<Box<Artist>>, QobuzApiError>`

Searches artists only.

#### `search_tracks(&self, query: &str, limit: Option<i32>, offset: Option<i32>) -> Result<ItemSearchResult<Box<Track>>, QobuzApiError>`

Searches tracks only.

#### `search_playlists(&self, query: &str, limit: Option<i32>, offset: Option<i32>) -> Result<ItemSearchResult<Box<Playlist>>, QobuzApiError>`

Searches playlists only.

### Content Browsing

#### `get_album(&self, album_id: &str, extra: Option<&str>) -> Result<Album, QobuzApiError>`

Retrieves album details. `extra` can include `"track_ids"` to get track ID list.

#### `get_artist(&self, artist_id: i32, extra: Option<&str>) -> Result<Artist, QobuzApiError>`

Retrieves artist details with optional extra fields.

#### `get_track(&self, track_id: i32) -> Result<Track, QobuzApiError>`

Retrieves track details.

#### `get_playlist(&self, playlist_id: &str, extra: Option<&str>) -> Result<Playlist, QobuzApiError>`

Retrieves playlist details.

#### `get_release_list(&self, artist_id: i32, limit: Option<i32>, offset: Option<i32>) -> Result<ItemSearchResult<Box<Album>>, QobuzApiError>`

Retrieves an artist's releases.

### Download

#### `get_track_file_url(&self, track_id: i32, format_id: i32) -> Result<FileUrl, QobuzApiError>`

Gets the download URL for a track at the specified quality.

**Pre-condition**: `user_auth_token` is `Some`.
**Format IDs**: 5 (MP3 320), 6 (FLAC 16/44.1), 7 (FLAC 24/96), 27 (FLAC 24/192). Named constants are provided in `src/models/file_url.rs` (e.g., `FORMAT_MP3`, `FORMAT_FLAC`, `FORMAT_HIRES_96`, `FORMAT_HIRES_192`).

#### `download_track(&self, track_id: i32, format_id: i32, output_dir: &Path, config: Option<&MetadataConfig>) -> Result<PathBuf, QobuzApiError>`

Downloads a single track. Embeds metadata if `config` is provided. Returns the path to the saved file.

**Filename format**: `{track_number:02}. {title}.{ext}`
**Error recovery**: On signature errors, refreshes credentials and retries once.

#### `download_album(&self, album_id: &str, format_id: i32, output_dir: &Path, config: Option<&MetadataConfig>, concurrency: Option<usize>) -> Result<Vec<PathBuf>, QobuzApiError>`

Downloads all tracks in an album concurrently. Returns paths to all saved files.

**Directory format**: `{output_dir}/{artist}/{album_title}/`
**Concurrency**: When `concurrency` is `None`, defaults to 4 simultaneous downloads. Uses `tokio::sync::Semaphore`.
**Error recovery**: On signature errors, refreshes credentials once and retries all failed tracks.
**Partial failure**: If individual tracks fail after all retry attempts, returns `DownloadError` listing failed track IDs. Successfully downloaded files remain on disk. The returned `Vec<PathBuf>` contains only paths of successfully downloaded tracks.

### Favorites

#### `add_user_favorites(&self, item_ids: &[i32], item_type: &str) -> Result<(), QobuzApiError>`

Adds items to favorites. `item_type` is `"album"`, `"artist"`, or `"track"`.

**Pre-condition**: `user_auth_token` is `Some`. Requires signed request.

#### `delete_user_favorites(&self, item_ids: &[i32], item_type: &str) -> Result<(), QobuzApiError>`

Removes items from favorites.

**Pre-condition**: `user_auth_token` is `Some`. Requires signed request.

#### `get_user_favorites(&self, item_type: &str, limit: Option<i32>, offset: Option<i32>) -> Result<UserFavorites, QobuzApiError>`

Retrieves the user's favorites list.

**Pre-condition**: `user_auth_token` is `Some`. Requires signed request.

#### `get_user_favorite_ids(&self) -> Result<UserFavorites, QobuzApiError>`

Retrieves only the favorite IDs grouped by type.

**Pre-condition**: `user_auth_token` is `Some`. Requires signed request.

---

## MetadataConfig

### `MetadataConfig::default() -> Self`

Returns default config with all fields enabled except `comment`.

### `MetadataConfig { field: bool, ... }`

All fields are public booleans. See data-model.md for complete field list.

---

## Error Handling Contract

All fallible operations return `Result<T, QobuzApiError>`. The error type implements `std::error::Error`, `Debug`, `Send`, `Sync`, and `'static`.

**Error categories**:
- `AuthenticationError` — Invalid credentials, expired tokens
- `HttpError` — Network failures, timeouts
- `ApiErrorResponse` — API returned error status
- `RateLimitError` — Rate limited (auto-retried up to 3 times before surfacing)
- `ResourceNotFoundError` — Content ID not found
- `DownloadError` — File download/creation failures
- `MetadataError` — Audio tagging failures
- `CredentialsError` — Credential loading/extraction failures
- `InvalidParameterError` — Invalid input parameters

**Retry behavior**: HTTP 429 and transient errors are automatically retried with exponential backoff (max 3 retries). Rate limit errors surface only after all retries exhausted.
