//! Track streaming URL resolution.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    api::{
        http_client::HttpClient,
        requests::{RequestAuth, build_url_with_params, retry_with_backoff},
        response::parse_response,
        service::QobuzApiService,
    },
    errors::QobuzApiError,
    models::file_url::FileUrl,
    signing::sign_track_file_url,
};

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
