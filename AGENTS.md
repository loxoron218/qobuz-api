---
name: code_agent
description: Senior Rust developer using modern idiomatic Rust for `qobuz-api`
---

# qobuz-api

Unofficial Rust client library for the Qobuz music streaming API, migrated from the C# implementation by DJDoubleD.

## Tech stack

- **Audio:** lofty
- **Concurrency:** async-channel, crossbeam, dynosaur, parking-lot, rayon, tokio, tokio-stream
- **Data & Persistence:** base64, md-5, serde, serde_json
- **HTTP:** reqwest
- **Utilities:** anyhow, criterion, regex, tempfile, thiserror, tracing, tracing-subscriber

## Codebase map (src/)

- `benches/` — criterion benchmarks (`hot_paths`)
- `metadata_tests/` — metadata fixture data and comparison reports
- `src/api/` — API client layer: auth (`auth/` — `proofs`), content (`content/` — `albums`, `artists`, `bundle`,
  `catalog`, `cover`, `discography`, `mixtape`, `persistence`, `playlists`, `stream`, `tracks`), favorites (`favorites/` —
  `collection`), http_client, requests (`requests/` — `dispatch`), response, retrieval, service (`service/` — `debug`),
  `fixture` (test-only)
- `src/api/macros.rs` — `delegate!` / `delegate_with_retry!` macros
- `src/credentials/` — `.env` I/O, `Credential` type, and web player credential extraction (`web`)
- `src/errors.rs` — typed `QobuzApiError` enum (`errors/` — `diagnostics` tests)
- `src/metadata/` — audio metadata extraction (`extractor.rs`), embedding (`embedder/` — `core`, `credits`, `dates`,
  `performers`), configuration (`config.rs`)
- `src/sanitize.rs` — cross-platform filename sanitization
- `src/signing.rs` — MD5-based request signature generation
- `tests/integration/` — offline stub HTTP client (`stub`) and live `live-tests` (`live/` binary: `live.rs` root with
  shared setup plus `login`, `browse`, `acquisition/` (`query`, `setup`), `search`, `conformance`, `verification/`
  (`comparison`, `exiftool`, `report`, `staging`))

## Conventions & workflow

- Read `CODING_STANDARDS.md` before writing code — single source of truth for style, error handling, concurrency, tracing, docs, testing, and
  build commands (lint, format, test, bench).
