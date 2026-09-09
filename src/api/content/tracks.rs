//! Track search, browse, and download operations.

use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering::Relaxed},
    time::Duration,
};

use {
    serde::{Deserialize, Serialize},
    serde_json::Value,
    tokio::time::sleep,
    tracing::{info, warn},
};

use crate::{
    api::{
        content::{
            albums::Album,
            artists::Artist,
            catalog::{ItemSearchResult, TrackSearchResponse},
            cover::fetch_track_cover,
            get_by_id,
            persistence::{
                DOWNLOAD_RETRY_BASE_DELAY_MS, MAX_DOWNLOAD_RETRIES, attempt_download,
                is_retryable_network_error,
            },
            search,
        },
        service::QobuzApiService,
    },
    errors::QobuzApiError::{self, Canceled},
    metadata::{
        config::MetadataConfig, embedder::embed_metadata_in_file,
        extractor::extract_comprehensive_metadata,
    },
    sanitize::sanitize_filename,
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

/// An individual music track.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Track {
    /// Unique track identifier.
    pub id: Option<i32>,
    /// Track title.
    pub title: Option<String>,
    /// Version subtitle.
    pub version: Option<String>,
    /// ISRC code.
    pub isrc: Option<String>,
    /// Track position in album.
    pub track_number: Option<i32>,
    /// Duration in seconds.
    pub duration: Option<i32>,
    /// Disc number.
    pub media_number: Option<i32>,
    /// Classical work title.
    pub work: Option<String>,
    /// Parent album.
    pub album: Option<Box<Album>>,
    /// Primary performer.
    pub performer: Option<Box<Artist>>,
    /// All performers (formatted string).
    pub performers: Option<String>,
    /// Primary composer.
    pub composer: Option<Box<Artist>>,
    /// Audio technical details.
    pub audio_info: Option<AudioInfo>,
    /// Copyright notice.
    pub copyright: Option<String>,
    /// Streaming available.
    pub streamable: Option<bool>,
    /// Download available.
    pub downloadable: Option<bool>,
    /// Hi-Res available.
    pub hires: Option<bool>,
    /// Max bit depth.
    pub maximum_bit_depth: Option<i32>,
    /// Max sample rate.
    pub maximum_sampling_rate: Option<f64>,
    /// Max channel count.
    pub maximum_channel_count: Option<i32>,
    /// Original release date.
    pub release_date_original: Option<String>,
    /// Streaming date.
    pub release_date_stream: Option<String>,
    /// Explicit content flag.
    pub parental_warning: Option<bool>,
    /// Sales metadata.
    pub product_sales_factors: Option<Value>,
}

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
                persistence::detect_partial_file,
                tracks::{get_track, search_tracks},
            },
            fixture::{MockServer, make_service},
        },
        assert_empty_search_test,
    };

    /// Tests search tracks deserializes results.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
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

    /// Tests search tracks empty results.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
    #[test]
    fn search_tracks_empty_results() -> Result<()> {
        assert_empty_search_test!(
            search_tracks,
            "Nothing",
            r#"{"tracks":{"items":[],"total":0}}"#
        );
        Ok(())
    }

    /// Tests get track by id.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
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

    /// Tests search tracks error response.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
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

    /// Tests get track not found.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
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

    /// Tests detect partial file returns none for missing.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
    #[test]
    fn detect_partial_file_returns_none_for_missing() -> Result<()> {
        let dir = TempDir::new()?;
        let path = dir.path().join("nonexistent.flac");
        ensure!(detect_partial_file(&path).is_none());
        Ok(())
    }

    /// Tests detect partial file returns size for existing.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
    #[test]
    fn detect_partial_file_returns_size_for_existing() -> Result<()> {
        let dir = TempDir::new()?;
        let path = dir.path().join("partial.flac");
        write(&path, b"hello world")?;
        ensure!(detect_partial_file(&path) == Some(11));
        Ok(())
    }

    /// Tests detect partial file returns none for empty.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
    #[test]
    fn detect_partial_file_returns_none_for_empty() -> Result<()> {
        let dir = TempDir::new()?;
        let path = dir.path().join("empty.flac");
        write(&path, b"")?;
        ensure!(detect_partial_file(&path).is_none());
        Ok(())
    }
}
