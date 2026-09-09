<!--
  Sync Impact Report
  ==================
  Version change: INITIAL → 1.0.0
  Modified principles: N/A (initial ratification)
  Added sections:
    - I. Zero-Compromise Code Quality
    - II. Test-First Engineering (Non-Negotiable)
    - III. Consistent User Experience
    - IV. Performance & Reliability
    - Performance & Reliability Standards
    - Development Workflow & Quality Gates
    - Governance
  Removed sections: N/A
  Templates requiring updates:
    - .specify/templates/plan-template.md — ✅ compatible (Constitution Check section present)
    - .specify/templates/spec-template.md — ✅ compatible (Requirements & Success Criteria align)
    - .specify/templates/tasks-template.md — ✅ compatible (phase/task structure aligns)
    - .specify/templates/checklist-template.md — ✅ compatible
    - .specify/templates/agent-file-template.md — ✅ compatible
  Follow-up TODOs: None
-->

# Qobuz API Rust Refactor Constitution

## Core Principles

### I. Zero-Compromise Code Quality

Every line of code MUST pass `cargo clippy -- -W clippy::pedantic` with zero
warnings. The project enforces strict clippy lints (deny-level in `Cargo.toml`)
including bans on `unwrap_used`, `expect_used`, `panic`, `dbg_macro`,
`unsafe` code, and `#[allow(clippy::xyz)]` attributes.

- Every `.rs` file MUST stay under 400 lines. When a file exceeds this limit,
  it MUST be split by capability/domain — never by the `models/handlers/utils`
  anti-pattern.
- Code MUST use only `.rs` files. `.ui`, `.xml`, and `.blp` files are
  forbidden; all UI MUST be built programmatically.
- Declarative macros (`macro_rules!`) MUST be used to eliminate duplication.
  Generics and abstractions MUST be preferred over copy-paste.
- Error handling MUST use `thiserror` for library crates and `anyhow` at the
  binary top-level only. `?` MUST be preferred over `match` chains. Errors
  MUST be `Send + Sync + 'static` in async Tokio tasks. `let _` and `.ok()`
  are forbidden — return errors with structured `tracing` context instead.
- All public items MUST have `///` documentation. All modules MUST have `//!`
  module-level docs. Function docs MUST include `# Arguments` and `# Returns`
  where applicable.

**Rationale**: The refactor exists because the original codebase accumulated
technical debt. Preventing regressions requires uncompromising quality gates
from day one.

### II. Test-First Engineering (Non-Negotiable)

Tests MUST be written before the implementation they verify, following a strict
Red-Green-Refactor cycle. No feature is considered complete until its tests pass.

- Unit tests MUST reside at the bottom of the file they test.
- Integration tests MUST verify API contract boundaries using deterministic
  mocks — never real network calls in CI.
- `tempfile` MUST be used for all test fixtures that touch the filesystem.
- Deterministic simulation testing MUST be used for technical infrastructure
  tasks (async schedulers, caches, retry logic).
- Every test MUST return `anyhow::Result` with `bail!` for assertions, or `()`
  for trivial cases.
- `cargo test` MUST pass before any commit. `cargo bench` MUST be run for any
  change that affects hot paths (HTTP pipelines, metadata embedding, streaming).

**Rationale**: The original `qobuz-api-rust` had Phase 11 ("Testing and
Validation") listed as incomplete. This refactor MUST NOT ship without
comprehensive test coverage from the start.

### III. Consistent User Experience

All user-facing behavior — whether CLI output, API responses, or GUI elements
— MUST follow predictable, documented patterns.

- Error messages presented to users MUST be actionable: they MUST state what
  went wrong and suggest a remediation step.
- Download paths, filename sanitization, and metadata tagging MUST be
  deterministic and consistent with the MusicBrainz Picard naming convention
  (`Artist/Album/NN. Title.ext`).
- Quality selection and format handling MUST present a uniform interface across
  album downloads, track downloads, and streaming operations.
- API service initialization and authentication MUST follow a single, documented
  flow. Environment variable names and `.env` handling MUST not change without
  a deprecation cycle.
- All user-facing strings MUST be free of raw error dumps. Structured
  `tracing` spans MUST be used internally; user output MUST be curated.

**Rationale**: The original CLI had inconsistent error handling (mix of
`println!` and error propagation), and the refactor aims to produce both a
library and a GUI application. Consistency across these surfaces prevents user
confusion and reduces support burden.

### IV. Performance & Reliability

The HTTP client pipeline, metadata processor, and download engine MUST meet
measurable performance and reliability targets. Performance regressions MUST be
caught by benchmarks before merge.

- HTTP request pipelines MUST reuse a single `reqwest::Client` instance with
  connection pooling enabled. No per-request client construction is allowed.
- Credential refresh MUST occur at most once per session, not per-track during
  album downloads. The original codebase's per-track refresh is a known
  performance defect that this refactor MUST eliminate.
- Async work MUST use `tokio` tasks for I/O-bound concurrency and `rayon` for
  CPU-bound parallelism (e.g., metadata tagging). Blocking the Tokio runtime
  is forbidden.
- Download operations MUST be resilient: transient HTTP errors MUST trigger
  configurable retries with exponential backoff. Partial downloads MUST be
  resumable.
- Memory allocations in hot paths MUST be profiled using `criterion` benchmarks
  before and after changes. No regression exceeding 10% in throughput or
  allocation count is acceptable without documented justification.
- Structured `tracing` with fields (e.g., `error!(error = %err, "Audio stream error")`)
  MUST be used for all observability. `println!` and `eprintln!` are forbidden
  in library code.

**Rationale**: The refactor targets a high-performance Rust client. The
original C# migration did not optimize for concurrency or credential caching.
This principle ensures the refactor delivers on Rust's performance promise.

## Performance & Reliability Standards

| Metric | Target | Enforcement |
|--------|--------|-------------|
| Clippy pedantic warnings | 0 | CI gate (`cargo clippy`) |
| File length | ≤ 400 lines | CI gate (custom check) |
| `unsafe` code | 0 occurrences | CI gate (`#![ forbid(unsafe_code) ]`) |
| Test pass rate | 100% | CI gate (`cargo test`) |
| Benchmark regression | ≤ 10% | CI advisory (`cargo bench`) |
| Album download credential refresh | ≤ 1 per session | Integration test |
| HTTP client instances | 1 per service | Code review + lint |
| Max nesting depth | 3 levels | `clippy.toml` `excessive-nesting-threshold` |

## Development Workflow & Quality Gates

1. **Pre-implementation**: Run `cargo clippy --fix --allow-dirty --all-targets -- -W clippy::pedantic && cargo fmt` before starting work.
2. **During implementation**: Write tests first (Red), implement (Green), then refactor. Run `cargo test` after every logical change.
3. **Pre-commit**: All tests MUST pass. `cargo clippy` MUST produce zero warnings. `cargo fmt` MUST produce no diff.
4. **Code review**: Every PR MUST verify compliance with all four principles. Any complexity beyond what is specified MUST be justified in the PR description with a simpler alternative considered and rejected.
5. **Documentation**: Use `Context7` MCP server for external library documentation before implementing features with unfamiliar dependencies. Never remove applicable existing documentation.

## Governance

This constitution is the authoritative source of non-negotiable rules for the
`qobuz-api` project. In conflicts between this document and any
other guidance (AGENTS.md, README, ad-hoc decisions), this constitution
prevails.

**Amendment procedure**:
- Any amendment MUST be proposed as a written change to this file with a
  migration plan for any existing code that violates the new rule.
- Amendments MUST update `LAST_AMENDED_DATE` and increment `CONSTITUTION_VERSION`
  per semantic versioning:
  - **MAJOR**: Principle removal or backward-incompatible redefinition.
  - **MINOR**: New principle or materially expanded guidance.
  - **PATCH**: Clarifications, wording fixes, non-semantic refinements.
- All PRs and code reviews MUST verify compliance with this constitution.

**Version**: 1.0.0 | **Ratified**: 2026-04-06 | **Last Amended**: 2026-04-06
