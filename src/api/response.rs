//! API response parsing, URL-encoded body truncation, and timestamp generation.

use std::time::{SystemTime, UNIX_EPOCH};

use {
    reqwest::Response,
    serde::de::DeserializeOwned,
    serde_json::{Value, from_str},
};

use crate::errors::QobuzApiError::{
    self, ApiErrorResponse, ApiResponseParseError, ResourceNotFoundError,
};

/// Parses an API response, handling error status codes and JSON deserialization.
///
/// # Arguments
///
/// * `response` - The HTTP response to parse
/// * `endpoint` - API endpoint path for error context
///
/// # Returns
///
/// The deserialized response body of type `T`.
///
/// # Errors
///
/// Returns a `QobuzApiError` on non-success status codes or JSON deserialization failure.
pub async fn parse_response<T: DeserializeOwned>(
    response: Response,
    endpoint: &str,
) -> Result<T, QobuzApiError> {
    let status = response.status();
    let body = response.text().await?;

    if status.is_success() {
        return from_str::<T>(&body).map_err(|e| ApiResponseParseError {
            content: truncate(&body, 500),
            source_info: Some(format!("{e}")),
        });
    }

    if let Ok(err) = from_str::<Value>(&body) {
        let code = i32::try_from(err.get("code").and_then(Value::as_i64).unwrap_or_default())
            .unwrap_or_default();
        let message = err
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown error");

        if status.as_u16() == 404 {
            return Err(ResourceNotFoundError {
                resource_type: endpoint.trim_start_matches('/').to_string(),
                resource_id: message.to_string(),
            });
        }

        return Err(ApiErrorResponse {
            code,
            message: message.to_string(),
            status: status.to_string(),
        });
    }

    Err(ApiErrorResponse {
        code: i32::from(status.as_u16()),
        message: truncate(&body, 200),
        status: status.to_string(),
    })
}

/// Generates a Unix timestamp string for request signing.
///
/// # Returns
///
/// The current Unix timestamp as a string.
#[must_use]
pub fn timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    now.as_secs().to_string()
}

/// Truncates a string to `max_len` characters with ellipsis.
///
/// # Arguments
///
/// * `s` - The string to truncate
/// * `max_len` - Maximum character length before truncation
///
/// # Returns
///
/// The original string if short enough, or a truncated version with `...`.
#[must_use]
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let mut truncated: String = s.chars().take(max_len).collect();
        truncated.push_str("...");
        truncated
    }
}
