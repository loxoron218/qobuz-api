# CODING STANDARDS

This document describes the coding style used in this project. It consolidates the rules from [`AGENTS.md`](./AGENTS.md) with the conventions observed
across the actual codebase. Formatting and linting rules are enforced by the following files:

- [`rustfmt.toml`](./rustfmt.toml)
- [`clippy.toml`](./clippy.toml)
- the `[lints.rust]` and `[lints.clippy]` tables in [`Cargo.toml`](./Cargo.toml) — notably `pedantic` and `nursery` groups are denied.

The core priorities are:

- high-performance, maintainable, idiomatic Rust code
- following modern best practices

These priorities take precedence over preserving existing code. **Do not be afraid of refactoring or API restructuring** when it serves them; write
clean, future-proof code rather than keeping an awkward interface unchanged.

---

## 1. Module & File Layout

### Capability-based grouping

Group modules by capability/domain. **Never** use generic structures like `models/`, `handlers/`, `utils/`, `types/`, or `common/`.

```text
src/
  domain_a/
    sub_a/
      item_a.rs
    sub_a.rs            # parent index for sub_a/
    item_b.rs
  domain_a.rs           # parent index for domain_a/
  domain_b/
    widget_a.rs
    widget_b.rs
  domain_b.rs           # parent index for domain_b/
```

### Module naming (global stem uniqueness)

Every word of a module file name is a stem, and all stems must be unique codebase-wide across `src/`, `tests/`, and `benches/`. Singular and plural
count as the same stem (`album` ≡ `albums`). A stem may not appear at **any word position** of any module name — neither as a leading word, a
trailing word, nor in between. Consequently, no two modules may share a word in any position (`track_row` + `track_transition`, `album_card` +
`album_playback`, `queue_persistence` + `settings_persistence`, or `playback::queue` + `ui::player::queue` are all forbidden). Stems are derived
exclusively from `.rs` file names (parent indexes included); grouping directories without a parent index (e.g. `tests/verification/`) contribute no
stems.

### Parent-index modules

Use the modern Rust module style: a `foo.rs` parent index that declares its submodules, with submodules living in a sibling `foo/` directory.

A parent index declares `pub mod` items and carries a `//!` module doc comment. It is **not** required to be a pure re-export shim: shared
implementation that doesn't belong to a single submodule — such as a module-level trait or a shared error enum — lives directly in `foo.rs` alongside
the `pub mod` declarations:

```rust
//! Persistence layer: domain types, repository trait, and error types.

pub mod active_tab;
pub mod catalog;
pub mod config;
pub mod database;
```

### Files

- **ONLY** write `.rs` files. Never use `.ui`, `.xml`, or `.blp` files.
- Keep each `.rs` file at **400 lines or fewer**. When a module outgrows the limit, split it into a subdirectory with a parent index.
- Keep module nesting shallow. The maximum sub-folder depth in the codebase is **2** (e.g. `src/ui/gallery/`).

---

## 2. Formatting & Linting

### Commands

```bash
cargo clippy --fix --allow-dirty --all-targets --all-features && cargo fmt
cargo test    # all tests must pass before committing
cargo bench   # benchmarks
```

### Hard rules

- **Never** commit with clippy warnings; treat every warning as an error.
- **Never** suppress lints with `#[allow(...)]` or `#[expect(...)]` attributes.

### Code style

- Prefer abstractions and generics over boilerplate code.
- Never hardcode values that should be configurable.

---

## 3. Imports

Imports are grouped into three blocks separated by blank lines, in this order:

1. `std::` items
2. external crates
3. `crate::` internal items

One item per import line (`imports_granularity = "One"`). Multiple external crates are imported in a single `use { ... }` block. Prefer
`crate::`-relative imports and nested re-imports.

```rust
use std::{fs::File, path::Path};

use {
    anyhow::{Context, Result},
    serde_json::{from_str, to_string_pretty},
    tracing::warn,
};

use crate::{
    domain_a::config::UserConfig,
    domain_b::{widget_a::WidgetA, widget_b::WidgetB},
};
```

Enum variants are often pulled in as nested imports:

```rust
use crate::domain_a::DomainError::{
    self, ParseError, IoError, NotFound,
};
```

When a naming collision would occur (e.g. an enum variant), prefer renaming the **import** with an alias rather than fully qualifying the name at
every call site. Alias the colliding import, not the item usage.

---

## 4. Error Handling

### Library crates: typed errors with `thiserror`

Define a `#[derive(Debug, Error)]` enum, document the enum with a summary comment, document **every variant** with `///`, give each a
`#[error("...")]` message, and use `#[from]` to wrap source errors.

```rust
/// Error type for repository operations.
#[derive(Debug, Error)]
pub enum RepoError {
    /// Database error.
    #[error("Database error: {0}")]
    Database(String),
    /// Entity not found.
    #[error("Entity not found: {0}")]
    NotFound(String),
}
```

### Binaries: `anyhow` at top level only

Use `anyhow::{Context, Result}` in the binary and at application boundaries. Attach context with `.context(...)` / `.with_context(...)` so errors are
actionable. Never leak `anyhow::Error` across library boundaries.

```rust
create_dir_all(&log_dir)
    .with_context(|| format!("Failed to create log directory: {}", log_dir.display()))?;
```

### Tests: `anyhow::Result` + `bail!` / `ensure!`

Functional tests return `anyhow::Result` and assert with `ensure!` / `bail!`. Trivial tests return `()` and use `assert!`.

```rust
#[test]
fn process_item_success() -> Result<()> {
    let service = make_service();
    let result = service.process(ItemId(1));
    ensure!(
        result == Some((2, PathBuf::from("/data/item_two"))),
        "expected the next item result"
    );
    Ok(())
}
```

### Rules

- Prefer `?` over `match` chains.
- In async (Tokio) code, errors **must** be `Send + Sync + 'static` in tasks.
- Never use `Box<dyn std::error::Error>` in libraries unless truly necessary.
- For simple recovery, use `if let Ok(..) else { ... }`.
- Never use `let _` or `.ok()` — return errors with context instead.
- Document error types with a summary comment and each variant with `///`.

---

## 5. Concurrency

Use `parking_lot::Mutex` (or `RwLock`) behind an `Arc`. Keep lock scopes minimal and explicit: wrap short critical sections in `{ ... }` blocks and
`drop(lock)` when a held lock should be released before further work.

Traits whose methods run on async tasks declare `Send + Sync + 'static` supertraits and return futures that are themselves `Send`:

```rust
/// Interface for all persistent storage operations.
pub trait Storage: Send + Sync + 'static {
    /// Insert a new track, returning its id.
    fn insert_track(&self, track: NewTrack) -> impl Future<Output = Result<i64>> + Send;
}
```

The common pattern is a struct holding `Arc<Mutex<Inner>>`:

```rust
/// Thread-safe work queue managing ordered item IDs with navigation.
#[derive(Debug, Clone)]
pub struct WorkQueue {
    /// Shared inner state protected by a mutex.
    inner: Arc<Mutex<WorkQueueInner>>,
}
```

Minimize the time locks are held — do not perform I/O or event dispatch while holding a lock:

```rust
{
    let mut state = shared.state.lock();
    state.current_id = Some(next_id);
    // ...
    drop(state);
}
```

---

## 6. Tracing & Observability

Use structured `tracing` everywhere, with fields for structured data. Never log with interpolated strings when fields are appropriate.

```rust
info!(item_id, "Advancing to next item",);

// errors carry the source as a field
warn!(
    error = %e,
    path = %config_path.display(),
    "Failed to parse config, falling back to defaults",
);
```

The binary entry point initializes `tracing-subscriber` with an `EnvFilter`, a JSON file appender, and a human-readable stderr layer.

---

## 7. Documentation

- **Module level:** begin every file with a `//!` module doc comment.
- **Public items:** document with `///`.
- **Function docs:** `# Arguments` and `# Returns` are required, but only where applicable.
- **Struct fields** and **enum variants**: document each with `///`.

```rust
//! Item processing engine.

/// Loads an item for processing.
///
/// # Arguments
///
/// * `item_path` - Path to the item file
///
/// # Returns
///
/// * `Result<Self, ParseError>` - Loaded item or parse error
pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, ParseError> { ... }
```

---

## 8. Testing & Benchmarking

- Place functional unit tests at the bottom of each file in a `#[cfg(test)] mod tests { ... }` block.
- Use `tempfile` for test fixtures.
- For technical tasks, use deterministic simulation testing.
- Integration/acceptance tests live in `tests/` and carry a `//!` header that references the relevant spec/FR (e.g.,
  `No-hardware-at-startup acceptance test (FR-030)`).
- Benchmarks live in `benches/` using `criterion`.
