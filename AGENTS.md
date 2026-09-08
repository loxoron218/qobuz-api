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
- `src/api/` — API client layer: auth (`auth/`), content (`content/` — search, browse, download), favorites, http_client, requests, response,
  service, service_download
- `src/api/macros.rs` — `delegate!` / `delegate_with_retry!` macros
- `src/api/test_support.rs` — mock HTTP client helpers
- `src/credentials/` — `.env` I/O and web player credential extraction
- `src/errors.rs` — typed `QobuzApiError` enum
- `src/metadata/` — audio metadata extraction (`extractor.rs`), embedding (`embedder/`), configuration (`config.rs`)
- `src/models/` — serde data models for API responses
- `src/sanitize.rs` — cross-platform filename sanitization
- `src/signing.rs` — MD5-based request signature generation
- `tests/integration/` — mock-HTTP integration tests and live `live-tests`

## Conventions & workflow

- Read `CODING_STANDARDS.md` before writing code — single source of truth for style, error handling, concurrency, tracing, docs, testing, and
  build commands (lint, format, test, bench).
