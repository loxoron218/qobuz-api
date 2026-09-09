//! Track streaming URL resolution.

/// Quality format IDs for track downloads.
pub mod quality {
    /// MP3 320kbps.
    pub const MP3_320: i32 = 5;
    /// FLAC 16-bit/44.1kHz (CD quality).
    pub const FLAC_16_44: i32 = 6;
    /// FLAC 24-bit/96kHz (Hi-Res).
    pub const FLAC_24_96: i32 = 7;
    /// FLAC 24-bit/192kHz (Hi-Res).
    pub const FLAC_24_192: i32 = 27;
}

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::{
    api::{
        http_client::HttpClient,
        requests::{RequestAuth, build_url_with_params, retry_with_backoff},
        response::parse_response,
        service::QobuzApiService,
    },
    errors::QobuzApiError,
    signing::sign_track_file_url,
};

/// Download URL for a track at a specific quality level.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct FileUrl {
    /// Track identifier.
    pub track_id: Option<i32>,
    /// Track duration in seconds.
    pub duration: Option<f64>,
    /// Download URL.
    pub url: Option<String>,
    /// Quality format ID.
    pub format_id: Option<i32>,
    /// MIME type of the audio file.
    pub mime_type: Option<String>,
    /// Sample rate in kHz.
    pub sampling_rate: Option<f64>,
    /// Bit depth.
    pub bit_depth: Option<i32>,
    /// Response status code.
    pub status: Option<i32>,
    /// Error message if applicable.
    pub message: Option<String>,
    /// Error code if applicable.
    pub code: Option<String>,
}

/// Gets the download URL for a track at the specified quality.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `track_id` - Track identifier
/// * `format_id` - Quality format ID (5=MP3, 6=FLAC 16-bit, 7=FLAC 24-bit/96kHz, 27=FLAC
///   24-bit/192kHz)
///
/// # Returns
///
/// The download URL and metadata for the track.
///
/// # Errors
///
/// Returns a `QobuzApiError` if not authenticated or the API request fails.
pub async fn get_track_file_url(
    service: &QobuzApiService,
    track_id: i32,
    format_id: i32,
) -> Result<FileUrl, QobuzApiError> {
    let token = service.require_auth_token()?;

    get_track_file_url_raw(
        service.http_client(),
        service.base_url(),
        &RequestAuth {
            app_id: &service.app_id,
            app_secret: service.app_secret(),
            user_auth_token: token,
        },
        track_id,
        format_id,
    )
    .await
}

/// Internal function to get a track file URL (used by download operations).
///
/// # Arguments
///
/// * `client` - HTTP client implementation
/// * `base_url` - API base URL
/// * `auth` - Application credentials and user authentication token
/// * `track_id` - Track identifier
/// * `format_id` - Quality format ID
///
/// # Returns
///
/// The signed download URL and metadata for the track.
///
/// # Errors
///
/// Returns a `QobuzApiError` if the signed API request fails.
pub async fn get_track_file_url_raw(
    client: &dyn HttpClient,
    base_url: &str,
    auth: &RequestAuth<'_>,
    track_id: i32,
    format_id: i32,
) -> Result<FileUrl, QobuzApiError> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string();

    let sig = sign_track_file_url(format_id, track_id, &ts, auth.app_secret);

    let params: Vec<(String, String)> = vec![
        ("track_id".to_string(), track_id.to_string()),
        ("format_id".to_string(), format_id.to_string()),
        ("intent".to_string(), "stream".to_string()),
        ("request_ts".to_string(), ts),
        ("request_sig".to_string(), sig),
        ("app_id".to_string(), auth.app_id.to_string()),
    ];

    let url = build_url_with_params(base_url, "/track/getFileUrl", &params);
    let response = retry_with_backoff(client, &url, auth.user_auth_token).await?;

    parse_response::<FileUrl>(response, "/track/getFileUrl").await
}

#[cfg(test)]
mod tests {
    use {
        anyhow::{Result, ensure},
        serde_json::from_str,
    };

    use crate::api::content::stream::{
        FileUrl,
        quality::{FLAC_16_44, FLAC_24_96, FLAC_24_192, MP3_320},
    };

    /// Tests quality constants are distinct.
    ///
    /// # Errors
    ///
    /// Returns an error if the assertion fails.
    #[test]
    fn quality_constants_are_distinct() -> Result<()> {
        ensure!(MP3_320 == 5, "MP3 constant mismatch");
        ensure!(FLAC_16_44 == 6, "FLAC 16/44 constant mismatch");
        ensure!(FLAC_24_96 == 7, "FLAC 24/96 constant mismatch");
        ensure!(FLAC_24_192 == 27, "FLAC 24/192 constant mismatch");
        Ok(())
    }

    /// Tests file URL deserializes from JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    #[test]
    fn file_url_deserializes() -> Result<()> {
        let body = r#"{"url":"https://example.com/file.flac","format_id":6}"#;
        let file_url: FileUrl = from_str(body)?;
        ensure!(
            file_url.url.as_deref() == Some("https://example.com/file.flac"),
            "URL mismatch"
        );
        ensure!(file_url.format_id == Some(6), "format mismatch");
        Ok(())
    }
}
