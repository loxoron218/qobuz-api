//! Metadata extraction from API models.

use crate::models::{
    album::{Album, Image},
    artist::Artist,
    track::Track,
};

/// Simplified album artist info for metadata passing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlbumArtistBrief {
    /// Artist name.
    pub name: Option<String>,
    /// Artist roles (e.g., "main-artist", "composer").
    pub roles: Option<Vec<String>>,
}

/// Comprehensive metadata extracted from API models.
///
/// Carries raw data needed by the embedder for format-specific processing.
/// The embedder performs complex extraction logic (artists from performers,
/// composers, format-specific handling) based on this data.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ComprehensiveMetadata {
    /// Track title.
    pub title: Option<String>,
    /// Track version (e.g., "Remastered 2015").
    pub track_version: Option<String>,
    /// Album title.
    pub album: Option<String>,
    /// Album version (e.g., "Deluxe Edition").
    pub album_version: Option<String>,
    /// Raw performers string from API (e.g., "Name, `MainArtist` - Other, Composer").
    pub performers: Option<String>,
    /// Track performer artist name.
    pub performer_name: Option<String>,
    /// Album primary artist name.
    pub album_artist_name: Option<String>,
    /// Album artists with roles for format-specific album artist handling.
    pub album_artists: Vec<AlbumArtistBrief>,
    /// Album composer name.
    pub album_composer_name: Option<String>,
    /// Track composer name.
    pub track_composer_name: Option<String>,
    /// Genre name.
    pub genre: Option<String>,
    /// Album download release date (highest priority).
    pub album_release_date_download: Option<String>,
    /// Album original release date.
    pub album_release_date_original: Option<String>,
    /// Track original release date.
    pub track_release_date_original: Option<String>,
    /// Album release timestamp (unix).
    pub released_at: Option<i64>,
    /// Copyright notice (from track).
    pub copyright: Option<String>,
    /// ISRC code.
    pub isrc: Option<String>,
    /// Universal Product Code.
    pub upc: Option<String>,
    /// Product URL for commercial information.
    pub product_url: Option<String>,
    /// Record label name.
    pub label: Option<String>,
    /// Release type (album, single, compilation, etc.).
    pub release_type: Option<String>,
    /// Product type (fallback for media type).
    pub product_type: Option<String>,
    /// Track number.
    pub track_number: Option<i32>,
    /// Total tracks in album.
    pub track_total: Option<i32>,
    /// Disc number.
    pub disc_number: Option<i32>,
    /// Total discs in album.
    pub disc_total: Option<i32>,
    /// Explicit content flag.
    pub parental_warning: Option<bool>,
    /// Whether track has a `work` field (classical music indicator).
    pub is_classical: bool,
    /// Best available cover art URL.
    pub cover_art_url: Option<String>,
    /// Downloaded cover art binary data.
    pub cover_art_data: Option<Vec<u8>>,
}

/// Selects the best available cover art URL from an [`Image`](crate::models::album::Image).
///
/// Resolution priority (highest to lowest): mega > extralarge > large > medium > thumbnail > small.
#[must_use]
pub fn best_cover_url(image: &Image) -> Option<String> {
    image
        .mega
        .clone()
        .or_else(|| image.extra_large.clone())
        .or_else(|| image.large.clone())
        .or_else(|| image.medium.clone())
        .or_else(|| image.thumbnail.clone())
        .or_else(|| image.small.clone())
}

/// Extracts comprehensive metadata from a track, its album, and artist.
///
/// Collects raw data from API models for the embedder to process with
/// format-specific logic.
///
/// # Arguments
///
/// * `track` - The track to extract metadata from
/// * `album` - Optional album containing the track
/// * `artist` - Optional artist for the track
///
/// # Returns
///
/// A `ComprehensiveMetadata` struct with all extracted fields.
pub fn extract_comprehensive_metadata(
    track: &Track,
    album: Option<&Album>,
    artist: Option<&Artist>,
) -> ComprehensiveMetadata {
    let performer_name = track
        .performer
        .as_ref()
        .and_then(|a| a.name.clone())
        .or_else(|| {
            let a = artist?;
            a.name.clone()
        });

    let album_artists: Vec<AlbumArtistBrief> = album
        .and_then(|a| a.artists.as_ref())
        .map(|artists| {
            artists
                .iter()
                .map(|a| AlbumArtistBrief {
                    name: a.name.clone(),
                    roles: a.roles.clone(),
                })
                .collect()
        })
        .unwrap_or_default();

    ComprehensiveMetadata {
        title: track.title.clone(),
        track_version: track.version.clone(),
        album: album.and_then(|a| a.title.clone()),
        album_version: album.and_then(|a| a.version.clone()),

        performers: track.performers.clone(),
        performer_name,
        album_artist_name: album
            .and_then(|a| a.artist.as_ref())
            .and_then(|a| a.name.clone()),
        album_artists,
        album_composer_name: album
            .and_then(|a| a.composer.as_ref())
            .and_then(|a| a.name.clone()),
        track_composer_name: track.composer.as_ref().and_then(|a| a.name.clone()),

        genre: album
            .and_then(|a| a.genre.as_ref())
            .and_then(|g| g.name.clone()),
        album_release_date_download: album.and_then(|a| a.release_date_download.clone()),
        album_release_date_original: album.and_then(|a| a.release_date_original.clone()),
        track_release_date_original: track.release_date_original.clone(),
        released_at: album.and_then(|a| a.released_at),
        copyright: track.copyright.clone(),
        isrc: track.isrc.clone(),
        upc: album.and_then(|a| a.upc.clone()),
        product_url: album.and_then(|a| a.product_url.clone()),
        label: album
            .and_then(|a| a.label.as_ref())
            .and_then(|l| l.name.clone()),
        release_type: album.and_then(|a| a.release_type.clone()),
        product_type: album.and_then(|a| a.product_type.clone()),
        track_number: track.track_number,
        track_total: album.and_then(|a| a.tracks_count),
        disc_number: track.media_number,
        disc_total: album.and_then(|a| a.media_count),
        parental_warning: track.parental_warning,
        is_classical: track.work.is_some(),

        cover_art_url: album
            .and_then(|a| a.image.as_ref())
            .and_then(best_cover_url),
        cover_art_data: None,
    }
}

#[cfg(test)]
mod tests {
    use {
        anyhow::{Result, ensure},
        serde_json::from_str,
    };

    use crate::{
        metadata::extractor::{best_cover_url, extract_comprehensive_metadata},
        models::{
            album::{Album, Image},
            track::Track,
        },
    };

    /// Makes a test track from JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    fn make_track() -> Result<Track> {
        let json =
            r#"{"id":1,"title":"So What","track_number":1,"media_number":1,"isrc":"USPJ11001001"}"#;
        Ok(from_str(json)?)
    }

    /// Makes a test album from JSON.
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails.
    fn make_album() -> Result<Album> {
        let json = r#"{"id":"123","title":"Kind of Blue","release_date_original":"1959-08-17","copyright":"(C) 1959","genre":{"id":1,"name":"Jazz"},"label":{"id":1,"name":"Columbia"},"artist":{"id":1,"name":"Miles Davis"},"image":{"small":"s","thumbnail":"t","medium":"m","large":"l","extralarge":"xl","mega":"mega"}}"#;
        Ok(from_str(json)?)
    }

    /// Tests extract all fields.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
    #[test]
    fn extract_all_fields() -> Result<()> {
        let track = make_track()?;
        let album = make_album()?;
        let meta = extract_comprehensive_metadata(&track, Some(&album), None);
        ensure!(meta.title.as_deref() == Some("So What"));
        ensure!(meta.album.as_deref() == Some("Kind of Blue"));
        ensure!(meta.album_artist_name.as_deref() == Some("Miles Davis"));
        ensure!(meta.genre.as_deref() == Some("Jazz"));
        ensure!(meta.album_release_date_original.as_deref() == Some("1959-08-17"));
        ensure!(meta.copyright.is_none());
        ensure!(meta.label.as_deref() == Some("Columbia"));
        ensure!(meta.track_number == Some(1));
        ensure!(meta.disc_number == Some(1));
        ensure!(meta.isrc.as_deref() == Some("USPJ11001001"));
        ensure!(meta.cover_art_url.as_deref() == Some("mega"));
        Ok(())
    }

    /// Tests extract with no album.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
    #[test]
    fn extract_with_no_album() -> Result<()> {
        let track = make_track()?;
        let meta = extract_comprehensive_metadata(&track, None, None);
        ensure!(meta.album.is_none());
        ensure!(meta.album_artist_name.is_none());
        ensure!(meta.genre.is_none());
        ensure!(meta.cover_art_url.is_none());
        ensure!(meta.title.as_deref() == Some("So What"));
        Ok(())
    }

    /// Tests cover art priority.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
    #[test]
    fn cover_art_priority() -> Result<()> {
        let image = Image {
            small: Some("s".into()),
            ..Image::default()
        };
        ensure!(best_cover_url(&image) == Some("s".to_string()));

        let image = Image {
            small: Some("s".into()),
            large: Some("l".into()),
            ..Image::default()
        };
        ensure!(best_cover_url(&image) == Some("l".to_string()));

        let image = Image::default();
        ensure!(best_cover_url(&image).is_none());
        Ok(())
    }

    /// Tests special characters preserved.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
    #[test]
    fn special_characters_preserved() -> Result<()> {
        let json = r#"{"id":4,"title":"Café Résumé — Über","track_number":1}"#;
        let track: Track = from_str(json)?;
        let meta = extract_comprehensive_metadata(&track, None, None);
        ensure!(meta.title.as_deref() == Some("Café Résumé — Über"));
        Ok(())
    }
}
