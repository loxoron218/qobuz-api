//! API response parsing, URL-encoded body truncation, and timestamp generation.

use std::time::{SystemTime, UNIX_EPOCH};

use {
    reqwest::Response,
    serde::{
        Deserialize, Deserializer,
        de::{DeserializeOwned, Error},
    },
    serde_json::{
        Value::{self, Null, Object, String as SerdeString},
        from_str, from_value,
    },
};

use crate::{
    api::content::albums::Image,
    errors::QobuzApiError::{self, ApiErrorResponse, ApiResponseParseError, ResourceNotFoundError},
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

/// Deserializes an optional JSON value.
///
/// # Arguments
///
/// * `deserializer` - The serde deserializer
///
/// # Returns
///
/// An optional JSON value.
///
/// # Errors
///
/// Returns a deserialization error if deserialization fails.
fn deserialize_optional_value<'de, D>(deserializer: D) -> Result<Option<Value>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<Value>::deserialize(deserializer)
}

/// Maps a deserialized value to a string, handling common cases.
///
/// # Arguments
///
/// * `value` - Optional JSON value to convert
///
/// # Returns
///
/// A string representation if conversion succeeds, or `None`.
fn map_value_to_string(value: Option<Value>) -> Option<String> {
    match value {
        None | Some(Null) => None,
        Some(SerdeString(s)) => Some(s),
        Some(v) => Some(v.to_string()),
    }
}

/// Deserializes fields that the API returns as either a plain string or `{"display":"Name"}`.
///
/// # Arguments
///
/// * `deserializer` - The serde deserializer
///
/// # Errors
///
/// Returns a deserialization error if the value is an invalid type.
///
/// # Returns
///
/// `Ok(Some(string))` for string values or objects with a `display` field,
/// `Ok(None)` for null/missing.
pub fn deserialize_flexible_name<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = deserialize_optional_value(deserializer)?;
    match value {
        None | Some(Null) => Ok(None),
        Some(SerdeString(s)) => Ok(Some(s)),
        Some(Object(map)) => map
            .get("display")
            .and_then(Value::as_str)
            .map(|s| Some(s.to_string()))
            .ok_or_else(|| Error::custom("object missing 'display' field")),
        Some(v) => Ok(map_value_to_string(Some(v))),
    }
}

/// Deserializes fields that the API returns as either a string, number, or other value.
///
/// # Arguments
///
/// * `deserializer` - The serde deserializer
///
/// # Errors
///
/// Returns a deserialization error if the value cannot be deserialized.
///
/// # Returns
///
/// `Ok(Some(string))` for string/number values, `Ok(None)` for null/missing.
pub fn deserialize_flexible_string_id<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = deserialize_optional_value(deserializer)?;
    Ok(map_value_to_string(value))
}

/// Deserializes picture/image fields the API returns as either a string URL, null, or an Image
/// object.
///
/// # Arguments
///
/// * `deserializer` - The serde deserializer
///
/// # Errors
///
/// Returns a deserialization error if the value is an invalid image object.
///
/// # Returns
///
/// `Ok(None)` for string URLs and null, `Ok(Some(Image))` for valid image objects.
pub fn deserialize_picture<'de, D>(deserializer: D) -> Result<Option<Image>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = deserialize_optional_value(deserializer)?;
    match value {
        None | Some(Null | SerdeString(_)) => Ok(None),
        Some(v) => from_value(v).map_err(Error::custom),
    }
}
