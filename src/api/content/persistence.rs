//! Download file I/O helpers: partial file detection, stream-to-disk writing, cover art fetch.

use std::{
    fs::{create_dir_all, metadata},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering::Relaxed},
};

use {
    reqwest::Response,
    tokio::{
        fs::{File, OpenOptions},
        io::AsyncWriteExt,
    },
    tokio_stream::StreamExt,
    tracing::warn,
};

use crate::{
    api::{
        content::{albums::Album, stream::get_track_file_url},
        requests::download_stream,
        service::QobuzApiService,
    },
    errors::QobuzApiError::{self, Canceled, DownloadError, HttpError},
};

/// Maximum number of retry attempts on download network errors.
pub const MAX_DOWNLOAD_RETRIES: u32 = 3;

/// Base delay in milliseconds for download retry exponential backoff.
pub const DOWNLOAD_RETRY_BASE_DELAY_MS: u64 = 2000;

/// Detects whether a partial file exists on disk with non-zero size.
///
/// # Arguments
///
/// * `path` - File path to check
///
/// # Returns
///
/// `Some(size)` if the file exists and has content, `None` otherwise.
#[must_use]
pub fn detect_partial_file(path: &Path) -> Option<u64> {
    let size = match metadata(path) {
        Ok(m) => m.len(),
        Err(_) => return None,
    };
    (size > 0).then_some(size)
}

/// Writes a streaming HTTP response to a file, optionally appending.
///
/// # Arguments
///
/// * `response` - HTTP response containing the audio stream
/// * `path` - Destination file path
/// * `append` - If `true`, append to existing file; otherwise create/overwrite
/// * `cancel` - Optional cancellation flag checked between chunks
///
/// # Returns
///
/// `Ok(())` on success.
///
/// # Errors
///
/// Returns a `QobuzApiError` on file I/O or stream read failures, or `Canceled` when the
/// cancellation flag is set.
pub async fn write_response_to_file(
    response: Response,
    path: &Path,
    append: bool,
    cancel: Option<&AtomicBool>,
) -> Result<(), QobuzApiError> {
    let mut file = if append {
        OpenOptions::new().append(true).open(path).await?
    } else {
        File::create(path).await?
    };
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        if cancel.is_some_and(|c| c.load(Relaxed)) {
            return Err(Canceled);
        }
        file.write_all(&chunk?).await?;
    }
    file.flush().await?;
    Ok(())
}

/// Saves a streaming response to disk with formatted filename.
///
/// # Arguments
///
/// * `response` - HTTP response containing the audio stream
/// * `track_id` - Track identifier (used in filename)
/// * `output_dir` - Directory to save the file
/// * `format_id` - Quality format ID (determines file extension)
/// * `append` - If `true`, append to existing file (resume); otherwise create new
/// * `cancel` - Optional cancellation flag checked during streaming
///
/// # Returns
///
/// The path to the saved file.
///
/// # Errors
///
/// Returns a `QobuzApiError` if directory creation, file I/O, or streaming fails.
pub async fn save_track_to_disk(
    response: Response,
    track_id: i32,
    output_dir: &Path,
    format_id: i32,
    append: bool,
    cancel: Option<&AtomicBool>,
) -> Result<PathBuf, QobuzApiError> {
    create_dir_all(output_dir)?;

    let ext = Album::extension_for_format(format_id);
    let path = output_dir.join(format!("{track_id:02}.{ext}"));
    write_response_to_file(response, &path, append, cancel).await?;
    Ok(path)
}

/// Performs a single download attempt, optionally resuming from a partial file.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `track_id` - Track identifier
/// * `format_id` - Quality format ID
/// * `path` - Destination file path
/// * `cancel` - Optional cancellation flag checked during streaming
///
/// # Returns
///
/// `Ok(true)` if the download resumed from a partial file, `Ok(false)` if fresh.
///
/// # Errors
///
/// Returns a `QobuzApiError` if the track file URL cannot be retrieved, the download stream
/// fails, or writing to disk fails.
pub async fn attempt_download(
    service: &QobuzApiService,
    track_id: i32,
    format_id: i32,
    path: &Path,
    cancel: Option<&AtomicBool>,
) -> Result<bool, QobuzApiError> {
    let offset = detect_partial_file(path);
    let range = offset.map(|s| format!("bytes={s}-"));

    let file_url = get_track_file_url(service, track_id, format_id).await?;

    let url = file_url.url.ok_or_else(|| DownloadError {
        message: format!("No download URL for track {track_id}"),
    })?;

    let token = service.require_auth_token()?;
    let response = download_stream(service.http_client(), &url, token, range.as_deref()).await?;

    let resumed = offset.is_some() && response.status().as_u16() == 206;
    if offset.is_some() && !resumed {
        warn!(
            track_id,
            path = %path.display(),
            "Server did not support Range request, re-downloading full file"
        );
    }

    write_response_to_file(response, path, resumed, cancel).await?;

    Ok(resumed)
}

/// Checks whether an error is a transient network failure worth retrying.
///
/// # Arguments
///
/// * `err` - The error to inspect
///
/// # Returns
///
/// `true` if the error is a connect timeout, body, or decode failure.
#[must_use]
pub fn is_retryable_network_error(err: &QobuzApiError) -> bool {
    let HttpError(reqwest_err) = err else {
        return false;
    };
    reqwest_err.is_connect()
        || reqwest_err.is_timeout()
        || reqwest_err.is_body()
        || reqwest_err.is_decode()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicBool;

    use {
        anyhow::{Result, anyhow, ensure},
        reqwest::Response,
        tempfile::TempDir,
        tokio::runtime::Runtime,
    };

    use crate::{
        api::{
            content::{
                cover::fetch_track_cover,
                persistence::{
                    DOWNLOAD_RETRY_BASE_DELAY_MS, MAX_DOWNLOAD_RETRIES, attempt_download,
                    is_retryable_network_error, save_track_to_disk, write_response_to_file,
                },
                stream::{get_track_file_url, quality::MP3_320},
            },
            fixture::{MockServer, make_service, make_service_without_auth},
        },
        errors::QobuzApiError::{AuthenticationError, Canceled, DownloadError},
        metadata::extractor::ComprehensiveMetadata,
    };

    /// Fetches a mock streaming response with its temp dir and runtime.
    ///
    /// # Returns
    ///
    /// Temp dir, runtime, and streaming response for file-write tests.
    ///
    /// # Errors
    ///
    /// Returns an error if the mock setup fails.
    fn mock_stream_response() -> Result<(TempDir, Runtime, Response)> {
        let server = MockServer::start(200, "audio-bytes")?;
        let service = make_service(&server.base_url())?;
        let rt = Runtime::new()?;
        let response = rt.block_on(service.http_client().get_with_auth(
            &format!("{}/stream", service.base_url()),
            "token",
            None,
        ))?;
        let dir = TempDir::new()?;
        Ok((dir, rt, response))
    }

    /// Tests retry constants have sensible values.
    ///
    /// # Errors
    ///
    /// Returns an error if the assertion fails.
    #[test]
    fn retry_constants_are_sensible() -> Result<()> {
        ensure!(MAX_DOWNLOAD_RETRIES > 0, "retries should be positive");
        ensure!(
            DOWNLOAD_RETRY_BASE_DELAY_MS > 0,
            "base delay should be positive"
        );
        Ok(())
    }

    /// Tests non-network errors are not retryable.
    ///
    /// # Errors
    ///
    /// Returns an error if the assertion fails.
    #[test]
    fn non_network_error_is_not_retryable() -> Result<()> {
        let err = Canceled;
        ensure!(
            !is_retryable_network_error(&err),
            "canceled should not retry"
        );
        let err = DownloadError {
            message: "boom".to_string(),
        };
        ensure!(
            !is_retryable_network_error(&err),
            "download should not retry"
        );
        Ok(())
    }

    /// Tests file URL fetching fails without authentication.
    ///
    /// # Errors
    ///
    /// Returns an error if the mock setup fails.
    #[test]
    fn file_url_requires_auth() -> Result<()> {
        let server = MockServer::start(200, "{}")?;
        let service = make_service_without_auth(&server.base_url())?;
        let rt = Runtime::new()?;
        let result = rt.block_on(get_track_file_url(&service, 1, MP3_320));
        ensure!(result.is_err(), "expected auth error");
        let err = result.err().ok_or_else(|| anyhow!("expected error"))?;
        ensure!(
            matches!(err, AuthenticationError { .. }),
            "expected AuthenticationError"
        );
        Ok(())
    }

    /// Tests stream writing saves bytes to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the mock setup fails.
    #[test]
    fn write_response_to_file_saves_bytes() -> Result<()> {
        let (dir, rt, response) = mock_stream_response()?;
        let path = dir.path().join("out.bin");
        rt.block_on(write_response_to_file(response, &path, false, None))?;
        ensure!(path.exists(), "output file should exist");
        Ok(())
    }

    /// Tests track saving creates an extensioned file.
    ///
    /// # Errors
    ///
    /// Returns an error if the mock setup fails.
    #[test]
    fn save_track_to_disk_creates_file() -> Result<()> {
        let (dir, rt, response) = mock_stream_response()?;
        let path = rt.block_on(save_track_to_disk(
            response,
            7,
            dir.path(),
            MP3_320,
            false,
            None,
        ))?;
        ensure!(path.exists(), "saved file should exist");
        Ok(())
    }

    /// Tests cover fetching returns none without a URL.
    ///
    /// # Errors
    ///
    /// Returns an error if the mock setup fails.
    #[test]
    fn fetch_cover_returns_none_without_url() -> Result<()> {
        let server = MockServer::start(200, "{}")?;
        let service = make_service(&server.base_url())?;
        let meta = ComprehensiveMetadata::default();
        let rt = Runtime::new()?;
        let cover = rt.block_on(fetch_track_cover(&service, &meta, "token"));
        ensure!(cover.is_none(), "expected no cover without URL");
        Ok(())
    }

    /// Tests download attempt fails without a file URL.
    ///
    /// # Errors
    ///
    /// Returns an error if the mock setup fails.
    #[test]
    fn attempt_download_errors_without_url() -> Result<()> {
        let body = r#"{"track_id":1}"#;
        let server = MockServer::start(200, body)?;
        let service = make_service(&server.base_url())?;
        let dir = TempDir::new()?;
        let path = dir.path().join("track.mp3");
        let rt = Runtime::new()?;
        let cancel: Option<&AtomicBool> = None;
        let result = rt.block_on(attempt_download(&service, 1, MP3_320, &path, cancel));
        ensure!(result.is_err(), "expected error without URL");
        Ok(())
    }
}
