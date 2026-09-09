//! API client layer: HTTP primitives, authentication, content operations, and favorites.

#[macro_use]
mod macros;
pub mod auth;
pub mod content;
pub mod favorites;
pub mod http_client;
pub mod requests;
pub mod response;
pub mod service;
pub mod service_download;

#[cfg(test)]
mod test_support;
