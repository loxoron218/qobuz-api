//! Download delegates for the central API service.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
};

use {tokio::runtime::Runtime, tracing::info};

use crate::{
    api::{
        content::{
            album_download::download_album, artist_download::download_artist,
            playlist_download::download_playlist, stream::get_track_file_url,
            tracks::download_track,
        },
        service::QobuzApiService,
    },
    errors::QobuzApiError,
    metadata::config::MetadataConfig,
    models::file_url::FileUrl,
};

impl QobuzApiService {
    delegate_with_retry!(pub fn get_track_file_url(track_id: i32, format_id: i32) -> FileUrl = get_track_file_url);

    delegate_with_retry_cancellable!(
        pub fn download_track_cancellable(track_id: i32, format_id: i32, output_dir: &Path, config: Option<&MetadataConfig>) -> PathBuf = download_track,
        cancel: Option<&AtomicBool>
    );

    delegate_with_retry_cancellable!(
        pub fn download_album_cancellable(album_id: &str, format_id: i32, output_dir: &Path, config: Option<&MetadataConfig>, concurrency: Option<usize>) -> Vec<PathBuf> = download_album,
        cancel: Option<Arc<AtomicBool>>
    );

    delegate_with_retry_cancellable!(
        pub fn download_artist_cancellable(artist_id: i32, format_id: i32, output_dir: &Path, config: Option<&MetadataConfig>, concurrency: Option<usize>) -> Vec<PathBuf> = download_artist,
        cancel: Option<Arc<AtomicBool>>
    );

    delegate_with_retry_cancellable!(
        pub fn download_playlist_cancellable(playlist_id: &str, format_id: i32, output_dir: &Path, config: Option<&MetadataConfig>) -> Vec<PathBuf> = download_playlist,
        cancel: Option<Arc<AtomicBool>>
    );

    /// Downloads a single track with cancellation support.
    ///
    /// # Arguments
    ///
    /// * `track_id` - Track identifier
    /// * `format_id` - Quality format ID
    /// * `output_dir` - Directory to save the downloaded file
    /// * `config` - Optional metadata configuration for tagging
    /// * `cancel` - Optional cancellation flag checked during the download
    ///
    /// # Returns
    ///
    /// The path to the downloaded file.
    ///
    /// # Errors
    ///
    /// Returns a `QobuzApiError` if URL retrieval, download, I/O fails, or `Canceled` if cancelled.
    pub fn download_track(
        &mut self,
        track_id: i32,
        format_id: i32,
        output_dir: &Path,
        config: Option<&MetadataConfig>,
    ) -> Result<PathBuf, QobuzApiError> {
        self.download_track_cancellable(track_id, format_id, output_dir, config, None)
    }

    /// Downloads an album.
    ///
    /// # Arguments
    ///
    /// * `album_id` - Album identifier
    /// * `format_id` - Quality format ID
    /// * `output_dir` - Base output directory for downloaded files
    /// * `config` - Optional metadata configuration for tagging
    /// * `concurrency` - Maximum number of concurrent track downloads
    ///
    /// # Returns
    ///
    /// A vector of paths to the downloaded track files.
    ///
    /// # Errors
    ///
    /// Returns a `QobuzApiError` if album retrieval, track download, or I/O fails.
    pub fn download_album(
        &mut self,
        album_id: &str,
        format_id: i32,
        output_dir: &Path,
        config: Option<&MetadataConfig>,
        concurrency: Option<usize>,
    ) -> Result<Vec<PathBuf>, QobuzApiError> {
        self.download_album_cancellable(album_id, format_id, output_dir, config, concurrency, None)
    }

    /// Downloads all albums by an artist.
    ///
    /// # Arguments
    ///
    /// * `artist_id` - Artist identifier
    /// * `format_id` - Quality format ID
    /// * `output_dir` - Base output directory for downloaded files
    /// * `config` - Optional metadata configuration for tagging
    /// * `concurrency` - Maximum number of concurrent track downloads per album
    ///
    /// # Returns
    ///
    /// A vector of paths to the downloaded track files.
    ///
    /// # Errors
    ///
    /// Returns a `QobuzApiError` if artist release list retrieval or any album download fails.
    pub fn download_artist(
        &mut self,
        artist_id: i32,
        format_id: i32,
        output_dir: &Path,
        config: Option<&MetadataConfig>,
        concurrency: Option<usize>,
    ) -> Result<Vec<PathBuf>, QobuzApiError> {
        self.download_artist_cancellable(
            artist_id,
            format_id,
            output_dir,
            config,
            concurrency,
            None,
        )
    }

    /// Downloads all tracks in a playlist.
    ///
    /// # Arguments
    ///
    /// * `playlist_id` - Playlist identifier
    /// * `format_id` - Quality format ID
    /// * `output_dir` - Base output directory for downloaded files
    /// * `config` - Optional metadata configuration for tagging
    ///
    /// # Returns
    ///
    /// A vector of paths to the downloaded track files.
    ///
    /// # Errors
    ///
    /// Returns a `QobuzApiError` if playlist retrieval, directory creation, or any track download
    /// fails.
    pub fn download_playlist(
        &mut self,
        playlist_id: &str,
        format_id: i32,
        output_dir: &Path,
        config: Option<&MetadataConfig>,
    ) -> Result<Vec<PathBuf>, QobuzApiError> {
        self.download_playlist_cancellable(playlist_id, format_id, output_dir, config, None)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, atomic::AtomicBool};

    use {
        anyhow::{Result, ensure},
        tempfile::TempDir,
    };

    use crate::{
        api::{
            service::QobuzApiService,
            test_support::{MockServer, make_service, make_service_without_auth},
        },
        models::file_url::quality::{FLAC_16_44, FLAC_24_96, MP3_320},
    };

    /// Creates an authenticated mock service with a temp dir for download tests.
    ///
    /// # Returns
    ///
    /// Authenticated service and temp dir for download tests.
    ///
    /// # Errors
    ///
    /// Returns an error if the mock setup fails.
    fn authenticated_service_with_dir() -> Result<(QobuzApiService, TempDir)> {
        let server = MockServer::start(200, "{}")?;
        let service = make_service(&server.base_url())?;
        let dir = TempDir::new()?;
        Ok((service, dir))
    }

    /// Creates an unauthenticated mock service with a temp dir for download tests.
    ///
    /// # Returns
    ///
    /// Unauthenticated service and temp dir for download tests.
    ///
    /// # Errors
    ///
    /// Returns an error if the mock setup fails.
    fn unauthenticated_service_with_dir() -> Result<(QobuzApiService, TempDir)> {
        let server = MockServer::start(200, "{}")?;
        let service = make_service_without_auth(&server.base_url())?;
        let dir = TempDir::new()?;
        Ok((service, dir))
    }

    /// Tests file URL retrieval fails without authentication.
    ///
    /// # Errors
    ///
    /// Returns an error if the mock setup fails.
    #[test]
    fn get_track_file_url_requires_auth() -> Result<()> {
        let server = MockServer::start(200, "{}")?;
        let mut service = make_service_without_auth(&server.base_url())?;
        let result = service.get_track_file_url(123, MP3_320);
        ensure!(result.is_err(), "expected auth error without token");
        Ok(())
    }

    /// Tests track download is cancelled when the flag is set.
    ///
    /// # Errors
    ///
    /// Returns an error if the mock setup fails.
    #[test]
    fn download_track_cancellable_returns_canceled() -> Result<()> {
        let (mut service, dir) = authenticated_service_with_dir()?;
        let cancel = AtomicBool::new(true);
        let result =
            service.download_track_cancellable(123, FLAC_16_44, dir.path(), None, Some(&cancel));
        ensure!(result.is_err(), "expected canceled error");
        Ok(())
    }

    /// Tests track download wrapper fails without authentication.
    ///
    /// # Errors
    ///
    /// Returns an error if the mock setup fails.
    #[test]
    fn download_track_requires_auth() -> Result<()> {
        let (mut service, dir) = unauthenticated_service_with_dir()?;
        let result = service.download_track(123, FLAC_24_96, dir.path(), None);
        ensure!(result.is_err(), "expected auth error without token");
        Ok(())
    }

    /// Tests album download is cancelled when the flag is set.
    ///
    /// # Errors
    ///
    /// Returns an error if the mock setup fails.
    #[test]
    fn download_album_cancellable_returns_canceled() -> Result<()> {
        let (mut service, dir) = authenticated_service_with_dir()?;
        let cancel = Arc::new(AtomicBool::new(true));
        let result = service.download_album_cancellable(
            "abc",
            MP3_320,
            dir.path(),
            None,
            None,
            Some(cancel),
        );
        ensure!(result.is_err(), "expected canceled error");
        Ok(())
    }

    /// Tests album download wrapper fails without authentication.
    ///
    /// # Errors
    ///
    /// Returns an error if the mock setup fails.
    #[test]
    fn download_album_requires_auth() -> Result<()> {
        let (mut service, dir) = unauthenticated_service_with_dir()?;
        let result = service.download_album("abc", MP3_320, dir.path(), None, None);
        ensure!(result.is_err(), "expected auth error without token");
        Ok(())
    }

    /// Tests artist download is cancelled when the flag is set.
    ///
    /// # Errors
    ///
    /// Returns an error if the mock setup fails.
    #[test]
    fn download_artist_cancellable_returns_canceled() -> Result<()> {
        let (mut service, dir) = authenticated_service_with_dir()?;
        let cancel = Arc::new(AtomicBool::new(true));
        let result =
            service.download_artist_cancellable(42, MP3_320, dir.path(), None, None, Some(cancel));
        ensure!(result.is_err(), "expected canceled error");
        Ok(())
    }

    /// Tests artist download wrapper fails without authentication.
    ///
    /// # Errors
    ///
    /// Returns an error if the mock setup fails.
    #[test]
    fn download_artist_requires_auth() -> Result<()> {
        let (mut service, dir) = unauthenticated_service_with_dir()?;
        let result = service.download_artist(42, MP3_320, dir.path(), None, None);
        ensure!(result.is_err(), "expected auth error without token");
        Ok(())
    }

    /// Tests playlist download is cancelled when the flag is set.
    ///
    /// # Errors
    ///
    /// Returns an error if the mock setup fails.
    #[test]
    fn download_playlist_cancellable_returns_canceled() -> Result<()> {
        let (mut service, dir) = authenticated_service_with_dir()?;
        let cancel = Arc::new(AtomicBool::new(true));
        let result =
            service.download_playlist_cancellable("pl123", MP3_320, dir.path(), None, Some(cancel));
        ensure!(result.is_err(), "expected canceled error");
        Ok(())
    }

    /// Tests playlist download wrapper fails without authentication.
    ///
    /// # Errors
    ///
    /// Returns an error if the mock setup fails.
    #[test]
    fn download_playlist_requires_auth() -> Result<()> {
        let (mut service, dir) = unauthenticated_service_with_dir()?;
        let result = service.download_playlist("pl123", MP3_320, dir.path(), None);
        ensure!(result.is_err(), "expected auth error without token");
        Ok(())
    }
}
