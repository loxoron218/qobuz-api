//! Artist download operations.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
};

use tracing::info;

use crate::{
    api::{
        content::{
            artists::get_release_list, bundle::download_album, check_cancel, fetch_with_cancel,
        },
        service::QobuzApiService,
    },
    errors::QobuzApiError::{self, DownloadError},
    metadata::config::MetadataConfig,
};

/// Downloads all albums by an artist.
///
/// Fetches the artist's release list and downloads each album sequentially.
/// Each album downloads its tracks concurrently.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `artist_id` - Artist identifier
/// * `format_id` - Quality format ID
/// * `output_dir` - Base output directory (subdirectories created per album)
/// * `config` - Optional metadata configuration for tagging
/// * `concurrency` - Maximum number of concurrent track downloads per album
/// * `cancel` - Optional cancellation flag
///
/// # Returns
///
/// A vector of paths to all downloaded track files.
///
/// # Errors
///
/// Returns a `QobuzApiError` if artist release list retrieval or any album download fails.
pub async fn download_artist(
    service: &QobuzApiService,
    artist_id: i32,
    format_id: i32,
    output_dir: &Path,
    config: Option<&MetadataConfig>,
    concurrency: Option<usize>,
    cancel: Option<Arc<AtomicBool>>,
) -> Result<Vec<PathBuf>, QobuzApiError> {
    let releases = fetch_with_cancel(cancel.as_deref(), || {
        get_release_list(service, artist_id, Some(50), None)
    })
    .await?;

    let album_items = releases.items.unwrap_or_default();
    if album_items.is_empty() {
        return Err(DownloadError {
            message: "No releases found for artist".to_string(),
        });
    }

    let mut all_paths = Vec::new();

    for album_info in &album_items {
        check_cancel(cancel.as_deref())?;

        let Some(album_id) = &album_info.id else {
            continue;
        };

        let paths = download_album(
            service,
            album_id,
            format_id,
            output_dir,
            config,
            concurrency,
            cancel.clone(),
        )
        .await?;

        all_paths.extend(paths);
    }

    info!(
        artist_id,
        count = all_paths.len(),
        "Artist download complete"
    );

    Ok(all_paths)
}
