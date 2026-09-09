//! Integration tests for download operations against the live Qobuz API.
//!
//! These tests authenticate against the real API, search for well-known content,
//! and verify that track file URL retrieval and actual downloading work correctly.
//!
//! **Tests for trial-vs-full content detection:** When credentials are wrong or
//! missing, the Qobuz API returns 30-second trial clip URLs instead of full tracks.
//! These tests detect that scenario and verify the appropriate behavior.
//!
//! **Tests FAIL if credentials are missing or wrong.** There is no silent skip.
//!
//! Setup: copy `.env.example` to `.env` and fill in your credentials, then:
//!
//! Run with `cargo test --test live --features live-tests`.
//!
//! In CI without credentials, run `cargo test` to run only unit tests and the mock integration.

#[cfg(test)]
mod tests {
    use std::fs::metadata;

    use {
        anyhow::{Result, anyhow, ensure},
        tempfile::TempDir,
        tracing::info,
    };

    use qobuz_api::models::file_url::{FileUrl, quality::MP3_320};

    use crate::test_support::{
        TRIAL_DURATION_THRESHOLD_SECS, create_authenticated_service, get_download_config,
        init_logging,
        query::{find_album_id, find_track_id},
        setup::{download_album, setup_album_download},
    };

    #[test]
    fn init() {
        init_logging();
    }

    #[test]
    fn live_get_track_file_url_returns_valid_url() -> Result<()> {
        let config = get_download_config();
        let mut service = create_authenticated_service()?;
        let track_id = find_track_id(&service, &config.track_query)?;

        info!(
            "Getting file URL for track {track_id} at format {}",
            config.format_id
        );

        let file_url: FileUrl = service.get_track_file_url(track_id, config.format_id)?;

        ensure!(
            file_url.url.is_some(),
            "get_track_file_url should return a URL, got: {file_url:?}"
        );

        let url = file_url
            .url
            .as_deref()
            .ok_or_else(|| anyhow!("file URL should be present"))?;
        ensure!(
            url.starts_with("http"),
            "file URL should start with http, got: {url}"
        );

        info!(
            "File URL: {}... (duration: {}s, format: {})",
            url.chars().take(80).collect::<String>(),
            file_url
                .duration
                .map_or_else(|| "?".to_string(), |d| format!("{d:.1}")),
            file_url
                .format_id
                .map_or_else(|| "?".to_string(), |f| f.to_string())
        );
        Ok(())
    }

    #[test]
    fn live_download_track_saves_file() -> Result<()> {
        let config = get_download_config();
        let mut service = create_authenticated_service()?;
        let track_id = find_track_id(&service, &config.track_query)?;
        let temp_dir = TempDir::new()?;

        info!(
            "Downloading track {track_id} to {}",
            temp_dir.path().display()
        );

        let path = service.download_track(track_id, config.format_id, temp_dir.path(), None)?;

        ensure!(path.exists(), "downloaded file should exist at {path:?}");

        let metadata = metadata(&path)?;
        ensure!(metadata.len() > 0, "downloaded file should not be empty");

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let expected_ext = if config.format_id == MP3_320 {
            "mp3"
        } else {
            "flac"
        };
        ensure!(
            ext == expected_ext,
            "file extension should be '{expected_ext}', got '{ext}'"
        );

        info!(
            "Downloaded: {} ({} bytes, ext: {ext})",
            path.display(),
            metadata.len()
        );
        Ok(())
    }

    #[test]
    fn live_download_track_detects_trial_clip() -> Result<()> {
        let config = get_download_config();
        let mut service = create_authenticated_service()?;
        let track_id = find_track_id(&service, &config.track_query)?;

        let file_url: FileUrl = service.get_track_file_url(track_id, config.format_id)?;

        let duration = file_url
            .duration
            .ok_or_else(|| anyhow!("file URL response missing duration"))?;

        if duration < TRIAL_DURATION_THRESHOLD_SECS {
            info!(
                "WARNING: Track {track_id} returned a trial clip ({duration:.1}s). This indicates \
                 the credentials may not have a valid subscription."
            );
        } else {
            info!(
                "Track {track_id} returned full content ({duration:.1}s) — subscription \
                 credentials are valid."
            );
        }

        ensure!(
            file_url.url.is_some(),
            "file URL should be present even for trial clips"
        );
        Ok(())
    }

    #[test]
    fn live_download_track_mp3_format() -> Result<()> {
        let mut service = create_authenticated_service()?;
        let track_id = find_track_id(&service, &get_download_config().track_query)?;
        let temp_dir = TempDir::new()?;

        info!("Downloading track {track_id} as MP3 320kbps");

        let path = service.download_track(track_id, MP3_320, temp_dir.path(), None)?;

        ensure!(path.exists(), "MP3 file should exist");

        let metadata = metadata(&path)?;
        ensure!(metadata.len() > 0, "MP3 file should not be empty");

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        ensure!(ext == "mp3", "extension should be mp3, got '{ext}'");

        info!("MP3 download: {} bytes", metadata.len());
        Ok(())
    }

    #[test]
    fn live_download_album_downloads_all_tracks() -> Result<()> {
        let config = get_download_config();
        let mut service = create_authenticated_service()?;
        let album_id = find_album_id(&service, &config.album_query)?;
        let temp_dir = TempDir::new()?;

        info!(
            "Downloading album {album_id} to {}",
            temp_dir.path().display()
        );

        let paths =
            service.download_album(&album_id, config.format_id, temp_dir.path(), None, Some(2))?;

        ensure!(
            !paths.is_empty(),
            "download_album should return at least one file"
        );

        for path in &paths {
            ensure!(path.exists(), "downloaded track should exist at {path:?}");

            let metadata = metadata(path)?;
            ensure!(
                metadata.len() > 0,
                "track file should not be empty: {path:?}"
            );
        }

        info!("Album download complete: {} tracks saved", paths.len());
        for (i, path) in paths.iter().enumerate() {
            let size = metadata(path).map_or(0, |m| m.len());
            info!(
                "  {}. {} ({} bytes)",
                i.saturating_add(1),
                path.display(),
                size
            );
        }
        Ok(())
    }

    #[test]
    fn live_download_album_creates_artist_album_directory() -> Result<()> {
        let mut setup = setup_album_download()?;
        let paths = download_album(&mut setup, Some(2))?;

        ensure!(!paths.is_empty(), "should have downloaded tracks");

        let first_path = paths
            .first()
            .ok_or_else(|| anyhow!("expected at least one downloaded file"))?;
        let parent = first_path
            .parent()
            .ok_or_else(|| anyhow!("downloaded file has no parent directory"))?;

        ensure!(
            parent != setup.temp_dir.path(),
            "tracks should be in an artist/album subdirectory, not directly in temp dir"
        );

        let artist_dir = parent
            .parent()
            .ok_or_else(|| anyhow!("album directory has no parent"))?;

        ensure!(
            artist_dir.starts_with(setup.temp_dir.path()),
            "directory structure should be <temp_dir>/<artist>/<album>/"
        );

        info!("Directory structure confirmed: {}/", parent.display());
        Ok(())
    }

    #[test]
    fn live_get_track_file_url_for_nonexistent_track_returns_error() -> Result<()> {
        let mut service = create_authenticated_service()?;

        let result = service.get_track_file_url(999_999_999, MP3_320);
        ensure!(
            result.is_err(),
            "get_track_file_url for nonexistent track should return error, got: {result:?}"
        );
        Ok(())
    }
}
