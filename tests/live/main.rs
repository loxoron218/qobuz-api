//! Live integration tests against the real Qobuz API.
//!
//! Run with `cargo test --test live --features live-tests`.

pub mod auth_tests;
pub mod browse_tests;
pub mod download_tests;
pub mod metadata_test;
pub mod metadata_tests;
pub mod search_tests;
pub mod test_support;
