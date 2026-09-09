//! Error types for the Qobuz API client.

#[cfg(test)]
pub mod diagnostics;

use std::io::Error as IoError;

use {reqwest::Error as ReqwestError, thiserror::Error};

/// Error type for all Qobuz API operations.
///
/// Implements `Send + Sync + 'static` for use in async Tokio tasks.
#[derive(Error, Debug)]
pub enum QobuzApiError {
    /// Authentication failure (invalid credentials, expired tokens).
    #[error("Authentication failed: {message}")]
    AuthenticationError {
        /// Error description.
        message: String,
    },

    /// HTTP request failure (network, timeout).
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] ReqwestError),

    /// I/O failure (file system, permissions).
    #[error("I/O error: {0}")]
    IoError(#[from] IoError),

    /// API returned an error response with code and message.
    #[error("API error {code}: {message}")]
    ApiErrorResponse {
        /// API error code.
        code: i32,
        /// API error message.
        message: String,
        /// API error status.
        status: String,
    },

    /// Failed to parse the API response body.
    #[error("Failed to parse API response: {content}")]
    ApiResponseParseError {
        /// The content that failed to parse.
        content: String,
        /// Optional source of the parse failure.
        source_info: Option<String>,
    },

    /// Service initialization failure.
    #[error("Service initialization failed: {message}")]
    InitializationError {
        /// Error description.
        message: String,
    },

    /// Invalid or missing credentials.
    #[error("Invalid credentials: {message}")]
    CredentialsError {
        /// Error description.
        message: String,
    },

    /// Download failure (file creation, network, disk).
    #[error("Download failed: {message}")]
    DownloadError {
        /// Error description.
        message: String,
    },

    /// Metadata tagging failure.
    #[error("Metadata error: {0}")]
    MetadataError(String),

    /// Requested resource not found by ID.
    #[error("{resource_type} not found: {resource_id}")]
    ResourceNotFoundError {
        /// Type of resource (album, artist, track, playlist).
        resource_type: String,
        /// Identifier that was not found.
        resource_id: String,
    },

    /// Rate limited by the API (retries exhausted).
    #[error("Rate limited: {message}")]
    RateLimitError {
        /// Error description.
        message: String,
    },

    /// Invalid parameter passed to an API method.
    #[error("Invalid parameter: {message}")]
    InvalidParameterError {
        /// Error description.
        message: String,
    },

    /// Unexpected API response structure or content.
    #[error("Unexpected API response: {message}")]
    UnexpectedApiResponseError {
        /// Error description.
        message: String,
    },

    /// Download was cancelled by the user.
    #[error("Download cancelled")]
    Canceled,
}
