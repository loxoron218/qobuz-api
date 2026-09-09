//! Download and credential test setup helpers.

use std::path::PathBuf;

use {
    anyhow::{Error, Result},
    tempfile::TempDir,
    tracing::info,
};

use qobuz_api::api::service::QobuzApiService;

use crate::test_support::{
    create_authenticated_service, get_download_config, query::find_album_id,
};

/// Setup for album download tests.
#[derive(Debug)]
pub struct AlbumDownloadSetup {
    /// Authenticated API service.
    pub service: QobuzApiService,
    /// Album ID to download.
    pub album_id: String,
    /// Temporary directory for downloaded files.
    pub temp_dir: TempDir,
    /// Audio format ID.
    pub format_id: i32,
}

/// Sets up an album download test.
///
/// # Returns
///
/// The album download setup if successful.
///
/// # Errors
///
/// Returns an error if authentication fails or the album cannot be found.
pub fn setup_album_download() -> Result<AlbumDownloadSetup> {
    let config = get_download_config();
    let service = create_authenticated_service()?;
    let album_id = find_album_id(&service, config.album_query())?;
    let temp_dir = TempDir::new()?;

    info!(
        "Downloading album {album_id} to {}",
        temp_dir.path().display()
    );

    Ok(AlbumDownloadSetup {
        service,
        album_id,
        temp_dir,
        format_id: config.format_id,
    })
}

/// Downloads an album using the provided setup.
///
/// # Arguments
///
/// * `setup` - The album download setup
/// * `max_tracks` - Optional maximum number of tracks to download
///
/// # Returns
///
/// A vector of downloaded file paths.
///
/// # Errors
///
/// Returns an error if the album download fails.
pub fn download_album(
    setup: &mut AlbumDownloadSetup,
    max_tracks: Option<usize>,
) -> Result<Vec<PathBuf>> {
    setup
        .service
        .download_album(
            &setup.album_id,
            setup.format_id,
            setup.temp_dir.path(),
            None,
            max_tracks,
        )
        .map_err(Error::from)
}
