//! Track download, cover art, JSON export, and directory setup for metadata tests.

use std::{
    fs::{create_dir_all, remove_file, rename, write},
    path::PathBuf,
};

use {
    anyhow::{Error, Result},
    reqwest::Client,
    serde_json::to_string_pretty,
    tokio::runtime::Runtime,
    tracing::info,
};

use qobuz_api::{
    api::service::QobuzApiService,
    metadata::{
        config::MetadataConfig, embedder::embed_metadata_in_file,
        extractor::extract_comprehensive_metadata,
    },
    models::{file_url::quality::MP3_320, track::Track},
    sanitize::sanitize_filename,
};

use crate::metadata_test::{
    TestTrack, downloads_dir, json_dir, metadata_dir, reports_dir, track_filename_base,
};

/// Downloads a test track, embeds metadata, and saves to the downloads directory.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `track` - Test track descriptor
/// * `format_id` - Quality format ID
///
/// # Returns
///
/// The path to the downloaded and tagged file.
///
/// # Errors
///
/// Returns an error if the download, tagging, or file setup fails.
pub fn download_test_track(
    service: &mut QobuzApiService,
    track: &TestTrack,
    format_id: i32,
) -> Result<PathBuf> {
    let format_label = if format_id == MP3_320 { "MP3" } else { "FLAC" };
    let ext = if format_id == MP3_320 { "mp3" } else { "flac" };
    let dl_dir = downloads_dir(&format_label.to_lowercase());
    let base = track_filename_base(track, format_label);
    let filename = format!("{base}.{ext}");
    let file_path = dl_dir.join(&filename);
    if file_path.exists() {
        remove_file(&file_path)?;
    }
    let downloaded = service.download_track(track.track_id, format_id, &dl_dir, None)?;
    let track_details: Track = service.get_track(track.track_id).map_err(Error::from)?;
    let album = track_details.album.as_deref();
    let mut meta = extract_comprehensive_metadata(&track_details, album, None);
    if let Some(url) = meta.cover_art_url.as_ref() {
        if let Ok(data) = download_cover_art(url) {
            meta.cover_art_data = Some(data);
        } else {
            info!(url = %url, "Failed to download cover art");
        }
    }
    let metadata_config = MetadataConfig::default();
    embed_metadata_in_file(&downloaded, &meta, &metadata_config)?;
    if downloaded != file_path {
        rename(&downloaded, &file_path)?;
    }
    info!(track = %track.title, format = format_label, "Downloaded and tagged");
    Ok(file_path)
}

/// Downloads cover art image data from a URL.
///
/// # Arguments
///
/// * `url` - URL of the cover art image
///
/// # Returns
///
/// `Ok(Vec<u8>)` on success with the raw image bytes, or a network error.
fn download_cover_art(url: &str) -> Result<Vec<u8>> {
    let rt = Runtime::new()?;
    rt.block_on(async {
        let client = Client::new();
        let resp = client.get(url).send().await?;
        resp.bytes().await.map(|b| b.to_vec()).map_err(Error::from)
    })
}

/// Saves track details from the API as pretty-printed JSON for reference.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `track` - Test track descriptor
///
/// # Returns
///
/// `Ok(())` on success, or an API / I/O error.
///
/// # Errors
///
/// Returns an error if the track cannot be fetched or written.
pub fn save_track_json(service: &QobuzApiService, track: &TestTrack) -> Result<()> {
    let dir = json_dir();
    let base = sanitize_filename(&format!(
        "{} - {} - {}",
        track.artist, track.title, track.track_id
    ));
    let path = dir.join(format!("{base}.json"));
    let track_details: Track = service.get_track(track.track_id).map_err(Error::from)?;
    let json = to_string_pretty(&track_details)?;
    write(&path, json)?;
    info!(track = %track.title, "JSON saved");
    Ok(())
}

/// Creates all required directories for metadata test artifacts.
///
/// # Returns
///
/// `Ok(())` on success, or an I/O error.
///
/// # Errors
///
/// Returns an error if a required directory cannot be created.
pub fn ensure_directories() -> Result<()> {
    for dir in [
        downloads_dir("flac"),
        downloads_dir("mp3"),
        metadata_dir("flac"),
        metadata_dir("mp3"),
        json_dir(),
        reports_dir(),
    ] {
        create_dir_all(&dir)?;
    }
    Ok(())
}
