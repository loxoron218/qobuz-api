//! Playlist download operations.

use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
};

use tracing::info;

use crate::{
    api::{
        content::{
            check_cancel, fetch_with_cancel, playlists::get_playlist, tracks::download_track,
        },
        service::QobuzApiService,
    },
    errors::QobuzApiError::{self, DownloadError},
    metadata::config::MetadataConfig,
    sanitize::sanitize_filename,
};

/// Downloads all tracks in a playlist.
///
/// Fetches the playlist with tracks included and downloads each track sequentially.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `playlist_id` - Playlist identifier
/// * `format_id` - Quality format ID
/// * `output_dir` - Base output directory (playlist name subdirectory is created)
/// * `config` - Optional metadata configuration for tagging
/// * `cancel` - Optional cancellation flag
///
/// # Returns
///
/// A vector of paths to all downloaded track files.
///
/// # Errors
///
/// Returns a `QobuzApiError` if playlist retrieval, directory creation, or any track download
/// fails.
pub async fn download_playlist(
    service: &QobuzApiService,
    playlist_id: &str,
    format_id: i32,
    output_dir: &Path,
    config: Option<&MetadataConfig>,
    cancel: Option<Arc<AtomicBool>>,
) -> Result<Vec<PathBuf>, QobuzApiError> {
    let playlist = fetch_with_cancel(cancel.as_deref(), || {
        get_playlist(service, playlist_id, Some("tracks"))
    })
    .await?;

    let title = playlist
        .name
        .unwrap_or_else(|| "Unknown Playlist".to_string());
    let dir = output_dir.join(sanitize_filename(&title));
    create_dir_all(&dir)?;

    let tracks = playlist.tracks.and_then(|t| t.items).unwrap_or_default();
    if tracks.is_empty() {
        return Err(DownloadError {
            message: "No tracks in playlist".to_string(),
        });
    }

    let mut all_paths = Vec::new();

    for track_item in &tracks {
        check_cancel(cancel.as_deref())?;

        let Some(track_id) = track_item.id else {
            continue;
        };

        let path = download_track(
            service,
            track_id,
            format_id,
            &dir,
            config,
            cancel.as_deref(),
        )
        .await?;

        all_paths.push(path);
    }

    info!(
        playlist_id,
        count = all_paths.len(),
        "Playlist download complete"
    );

    Ok(all_paths)
}
