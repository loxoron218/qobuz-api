//! Album data model.

use {
    serde::{Deserialize, Serialize},
    serde_json::Value,
};

use crate::models::artist::Artist;

/// A music album with full metadata.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Album {
    /// Unique album identifier.
    pub id: Option<String>,
    /// Album title.
    pub title: Option<String>,
    /// Version/subtitle.
    pub version: Option<String>,
    /// Universal Product Code.
    pub upc: Option<String>,
    /// Qobuz web URL.
    pub url: Option<String>,
    /// Primary artist.
    pub artist: Option<Box<Artist>>,
    /// All artists.
    pub artists: Option<Vec<Box<Artist>>>,
    /// Primary composer.
    pub composer: Option<Box<Artist>>,
    /// Record label.
    pub label: Option<Label>,
    /// Primary genre.
    pub genre: Option<Genre>,
    /// All genres.
    pub genres: Option<Vec<Genre>>,
    /// Cover art URLs.
    pub image: Option<Image>,
    /// Total duration in seconds.
    pub duration: Option<i32>,
    /// Number of tracks.
    pub tracks_count: Option<i32>,
    /// Number of discs.
    pub media_count: Option<i32>,
    /// Original release date.
    pub release_date_original: Option<String>,
    /// Streaming availability date.
    pub release_date_stream: Option<String>,
    /// Download availability date.
    pub release_date_download: Option<String>,
    /// Product type identifier.
    pub product_type: Option<String>,
    /// Release type (album, single, etc.).
    pub release_type: Option<String>,
    /// Hi-Res audio available.
    pub hires: Option<bool>,
    /// Hi-Res streaming available.
    pub hires_streamable: Option<bool>,
    /// Download available.
    pub downloadable: Option<bool>,
    /// Streaming available.
    pub streamable: Option<bool>,
    /// List of track IDs (when requested with extra).
    pub track_ids: Option<Vec<i32>>,
    /// Product URL for commercial information.
    pub product_url: Option<String>,
    /// Unix timestamp of when the album was released.
    pub released_at: Option<i64>,
    /// Copyright notice.
    pub copyright: Option<String>,
    /// Sales metadata.
    pub product_sales_factors: Option<Value>,
    /// Maximum available bit depth.
    pub maximum_bit_depth: Option<i32>,
    /// Maximum available sample rate.
    pub maximum_sampling_rate: Option<f64>,
    /// Maximum channel count.
    pub maximum_channel_count: Option<i32>,
}

impl Album {
    /// Returns the file extension for the given quality format ID.
    ///
    /// # Arguments
    ///
    /// * `format_id` - Quality format ID (5=MP3, everything else=FLAC)
    ///
    /// # Returns
    ///
    /// The file extension as a static string slice (`"mp3"` or `"flac"`).
    #[must_use]
    pub const fn extension_for_format(format_id: i32) -> &'static str {
        match format_id {
            5 => "mp3",
            _ => "flac",
        }
    }
}

/// Music genre.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Genre {
    /// Genre ID.
    pub id: Option<i32>,
    /// Genre name.
    pub name: Option<String>,
    /// URL-friendly name.
    pub slug: Option<String>,
    /// Display color.
    pub color: Option<String>,
}

/// Cover art URLs in multiple sizes.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Image {
    /// Small thumbnail URL.
    pub small: Option<String>,
    /// Thumbnail URL.
    pub thumbnail: Option<String>,
    /// Medium size URL.
    pub medium: Option<String>,
    /// Large size URL.
    pub large: Option<String>,
    /// Extra-large URL.
    #[serde(rename = "extralarge")]
    pub extra_large: Option<String>,
    /// Highest resolution URL.
    pub mega: Option<String>,
    /// Back cover URL.
    pub back: Option<String>,
    /// Generic/poster URL (returned by some endpoints instead of the size-specific map).
    pub url: Option<String>,
}

/// Record label.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Label {
    /// Label ID.
    pub id: Option<i32>,
    /// Label name.
    pub name: Option<String>,
    /// URL-friendly name.
    pub slug: Option<String>,
}

#[cfg(test)]
mod tests {
    use anyhow::{Result, ensure};

    use crate::models::album::Album;

    /// Tests file extension mapping for quality IDs.
    ///
    /// # Errors
    ///
    /// Returns an error if the assertion fails.
    #[test]
    fn extension_for_format_maps_types() -> Result<()> {
        ensure!(
            Album::extension_for_format(5) == "mp3",
            "MP3 should map to mp3"
        );
        ensure!(
            Album::extension_for_format(6) == "flac",
            "FLAC should map to flac"
        );
        ensure!(
            Album::extension_for_format(27) == "flac",
            "hi-res should map to flac"
        );
        Ok(())
    }
}
