//! Central API service facade: collection operation mixins for downloads and lookup.
//!
//! The [`QobuzApiService`](crate::api::service_state::QobuzApiService) state
//! lives in [`service_state`](crate::api::service_state); the modules below
//! add download, discovery, library, and lookup operations.

pub mod debug;
pub mod discovery;
pub mod library;
pub mod lookup;
