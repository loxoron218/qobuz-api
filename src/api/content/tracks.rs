//! Track search, browse, and download operations.

use std::{
    convert::AsRef,
    fs::create_dir_all,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering::Relaxed},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use {
    tokio::time::sleep,
    tracing::{info, warn},
};

use crate::{
    api::{
        content::{
            download_io::{
                DOWNLOAD_RETRY_BASE_DELAY_MS, MAX_DOWNLOAD_RETRIES, attempt_download,
                fetch_track_cover, is_retryable_network_error,
            },
            get_by_id, search,
        },
        http_client::HttpClient,
        requests::{RequestAuth, build_url_with_params, retry_with_backoff},
        response::parse_response,
        service::QobuzApiService,
    },
    errors::QobuzApiError::{self, Canceled},
    metadata::{
        config::MetadataConfig, embedder::embed_metadata_in_file,
        extractor::extract_comprehensive_metadata,
    },
    models::{
        album::Album,
        file_url::FileUrl,
        search::{ItemSearchResult, TrackSearchResponse},
        track::Track,
    },
    sanitize::sanitize_filename,
    signing::sign_track_file_url,
};

/// Searches for tracks matching the query.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `query` - Search query string
/// * `limit` - Maximum number of results to return
/// * `offset` - Pagination offset
///
/// # Returns
///
/// A paginated `ItemSearchResult` containing matching tracks.
///
/// # Errors
///
/// Returns a `QobuzApiError` if not authenticated or the API request fails.
pub async fn search_tracks(
    service: &QobuzApiService,
    query: &str,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<ItemSearchResult<Box<Track>>, QobuzApiError> {
    let resp: TrackSearchResponse = search(service, "/track/search", query, limit, offset).await?;
    Ok(resp.tracks)
}

/// Retrieves track details by ID.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `track_id` - Track identifier
///
/// # Returns
///
/// The track details.
///
/// # Errors
///
/// Returns a `QobuzApiError` if not authenticated or the API request fails.
pub async fn get_track(service: &QobuzApiService, track_id: i32) -> Result<Track, QobuzApiError> {
    get_by_id(service, "/track/get", "track_id", track_id, None).await
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

/// Downloads a single track to the specified directory.
///
/// Retries up to [`MAX_DOWNLOAD_RETRIES`] times on transient network errors,
/// resuming from the partial file on disk via HTTP Range requests.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `track_id` - Track identifier
/// * `format_id` - Quality format ID
/// * `output_dir` - Directory to save the downloaded file
/// * `config` - Optional metadata configuration for tagging
///
/// # Returns
///
/// The path to the downloaded file.
///
/// # Errors
///
/// Returns a `QobuzApiError` if URL retrieval, download, or I/O fails.
pub async fn download_track(
    service: &QobuzApiService,
    track_id: i32,
    format_id: i32,
    output_dir: &Path,
    config: Option<&MetadataConfig>,
    cancel: Option<&AtomicBool>,
) -> Result<PathBuf, QobuzApiError> {
    if cancel.is_some_and(|c| c.load(Relaxed)) {
        return Err(Canceled);
    }

    let track = get_track(service, track_id).await?;

    if cancel.is_some_and(|c| c.load(Relaxed)) {
        return Err(Canceled);
    }

    let ext = Album::extension_for_format(format_id);
    let track_num = track.track_number.unwrap_or(track_id);
    let title = track.title.as_deref().unwrap_or("Unknown");
    let safe_name = sanitize_filename(&format!("{track_num:02}. {title}"));
    let filename = format!("{safe_name}.{ext}");

    create_dir_all(output_dir)?;
    let path = output_dir.join(&filename);

    let mut resumed = false;

    for attempt in 0..=MAX_DOWNLOAD_RETRIES {
        if cancel.is_some_and(|c| c.load(Relaxed)) {
            return Err(Canceled);
        }

        match attempt_download(service, track_id, format_id, &path, cancel).await {
            Ok(r) => {
                resumed = r;
                break;
            }
            Err(e) if is_retryable_network_error(&e) && attempt < MAX_DOWNLOAD_RETRIES => {
                let delay = DOWNLOAD_RETRY_BASE_DELAY_MS.saturating_mul(2u64.pow(attempt));
                warn!(
                    track_id,
                    attempt,
                    delay_ms = delay,
                    path = %path.display(),
                    error = %e,
                    "Download failed, retrying with resume"
                );
                sleep(Duration::from_millis(delay)).await;
            }
            Err(e) => return Err(e),
        }
    }

    if cancel.is_some_and(|c| c.load(Relaxed)) {
        return Err(Canceled);
    }

    info!(
        track_id,
        format_id,
        path = %path.display(),
        resumed,
        "Track downloaded"
    );

    if let Some(cfg) = config {
        let token = service.require_auth_token()?;
        let album_info = track.album.as_ref().map(AsRef::as_ref);
        let mut meta = extract_comprehensive_metadata(&track, album_info, None);

        meta.cover_art_data = fetch_track_cover(service, &meta, token).await;
        embed_metadata_in_file(&path, &meta, cfg)?;
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use std::fs::write;

    use {
        anyhow::{Result, anyhow, ensure},
        tempfile::TempDir,
        tokio::runtime::Runtime,
    };

    use crate::{
        api::{
            content::{
                download_io::detect_partial_file,
                tracks::{get_track, search_tracks},
            },
            test_support::{MockServer, make_service},
        },
        assert_empty_search_test,
    };

    #[test]
    fn search_tracks_deserializes_results() -> Result<()> {
        let body = r#"{"tracks":{"items":[{"id":1,"title":"So What"}],"total":1}}"#;
        let server = MockServer::start(200, body)?;
        let service = make_service(&server.base_url())?;
        let rt = Runtime::new()?;
        let result = rt.block_on(search_tracks(&service, "So What", Some(5), None))?;
        let items = result.items.ok_or_else(|| anyhow!("no items"))?;
        ensure!(items.len() == 1);
        ensure!(items.first().and_then(|i| i.title.as_deref()) == Some("So What"));
        Ok(())
    }

    #[test]
    fn search_tracks_empty_results() -> Result<()> {
        assert_empty_search_test!(
            search_tracks,
            "Nothing",
            r#"{"tracks":{"items":[],"total":0}}"#
        );
        Ok(())
    }

    #[test]
    fn get_track_by_id() -> Result<()> {
        let body = r#"{"id":42,"title":"Blue in Green"}"#;
        let server = MockServer::start(200, body)?;
        let service = make_service(&server.base_url())?;
        let rt = Runtime::new()?;
        let track = rt.block_on(get_track(&service, 42))?;
        ensure!(track.title.as_deref() == Some("Blue in Green"));
        Ok(())
    }

    #[test]
    fn search_tracks_error_response() -> Result<()> {
        let body = r#"{"status":"error","code":500,"message":"Server error"}"#;
        let server = MockServer::start(500, body)?;
        let service = make_service(&server.base_url())?;
        let rt = Runtime::new()?;
        let result = rt.block_on(search_tracks(&service, "fail", None, None));
        ensure!(result.is_err());
        Ok(())
    }

    #[test]
    fn get_track_not_found() -> Result<()> {
        let body = r#"{"status":"error","code":404,"message":"Track not found"}"#;
        let server = MockServer::start(404, body)?;
        let service = make_service(&server.base_url())?;
        let rt = Runtime::new()?;
        let result = rt.block_on(get_track(&service, 99999));
        ensure!(result.is_err());
        Ok(())
    }

    #[test]
    fn detect_partial_file_returns_none_for_missing() -> Result<()> {
        let dir = TempDir::new()?;
        let path = dir.path().join("nonexistent.flac");
        ensure!(detect_partial_file(&path).is_none());
        Ok(())
    }

    #[test]
    fn detect_partial_file_returns_size_for_existing() -> Result<()> {
        let dir = TempDir::new()?;
        let path = dir.path().join("partial.flac");
        write(&path, b"hello world")?;
        ensure!(detect_partial_file(&path) == Some(11));
        Ok(())
    }

    #[test]
    fn detect_partial_file_returns_none_for_empty() -> Result<()> {
        let dir = TempDir::new()?;
        let path = dir.path().join("empty.flac");
        write(&path, b"")?;
        ensure!(detect_partial_file(&path).is_none());
        Ok(())
    }
}
