//! Shared media primitives: flexible deserializers plus image, genre, and audio details.
//!
//! Leaf module with no `crate::api` imports so record entities depend downward
//! without cycles.

use {
    serde::{Deserialize, Deserializer, Serialize, de::Error},
    serde_json::{
        Value,
        Value::{Null, Object},
        from_value,
    },
};

/// Audio technical details.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct AudioInfo {
    /// Bit depth.
    pub bit_depth: Option<i32>,
    /// Sample rate in kHz.
    pub sampling_rate: Option<f64>,
    /// Channel count.
    pub channels: Option<i32>,
    /// Audio codec.
    pub codec: Option<String>,
}

/// Artist biography text.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Biography {
    /// Biography text content.
    pub text: Option<String>,
    /// Summary text.
    pub summary: Option<String>,
}

/// Music genre.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Genre {
    /// Genre ID.
    pub id: Option<i32>,
    /// Genre name.
    pub name: Option<String>,
    /// URL-friendly name.
    pub slug: Option<String>,
    /// Display color.
    pub color: Option<String>,
}

/// Cover art URLs in multiple sizes.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Image {
    /// Small thumbnail URL.
    pub small: Option<String>,
    /// Thumbnail URL.
    pub thumbnail: Option<String>,
    /// Medium size URL.
    pub medium: Option<String>,
    /// Large size URL.
    pub large: Option<String>,
    /// Extra-large URL.
    #[serde(rename = "extralarge")]
    pub extra_large: Option<String>,
    /// Highest resolution URL.
    pub mega: Option<String>,
    /// Back cover URL.
    pub back: Option<String>,
    /// Generic/poster URL (returned by some endpoints instead of the size-specific map).
    pub url: Option<String>,
}

/// Record label.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Label {
    /// Label ID.
    pub id: Option<i32>,
    /// Label name.
    pub name: Option<String>,
    /// URL-friendly name.
    pub slug: Option<String>,
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
    if let Some(s) = value.as_ref().and_then(|v| v.as_str()) {
        return Ok(Some(s.to_string()));
    }
    match value {
        None | Some(Null) => Ok(None),
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
/// Returns a deserialization error if the value cannot be deserialized.
fn deserialize_optional_value<'de, D>(deserializer: D) -> Result<Option<Value>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<Value>::deserialize(deserializer)
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
        None | Some(Null) => Ok(None),
        Some(v) if v.is_string() => Ok(None),
        Some(v) => from_value(v).map_err(Error::custom),
    }
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
        Some(v) => Some(
            v.as_str()
                .map_or_else(|| v.to_string(), ToString::to_string),
        ),
    }
}
