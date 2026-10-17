//! Music catalog records: album, artist, track, and playlist entities.
//!
//! Shared data types used across content operations, favorites, and metadata extraction.

use {
    serde::{Deserialize, Serialize},
    serde_json::Value,
};

use crate::{
    api::content::record_media::{
        AudioInfo, Biography, Genre, Image, Label, deserialize_flexible_name,
        deserialize_flexible_string_id, deserialize_picture,
    },
    credentials::Credential,
};

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

/// API response wrapper for album search (`/album/search`).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct AlbumSearchResponse {
    /// Album search results.
    pub albums: ItemSearchResult<Box<Album>>,
}

/// A music artist.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Artist {
    /// Unique artist identifier.
    pub id: Option<i32>,
    /// Artist name (may be a plain string or `{"display":"Name"}` from the API).
    #[serde(default, deserialize_with = "deserialize_flexible_name")]
    pub name: Option<String>,
    /// URL-friendly name.
    pub slug: Option<String>,
    /// Artist picture (may be a string URL or an Image object from the API).
    #[serde(default, deserialize_with = "deserialize_picture")]
    pub picture: Option<Image>,
    /// Artist image (alternate field).
    pub image: Option<Image>,
    /// Biography text.
    pub biography: Option<Biography>,
    /// Number of albums.
    pub albums_count: Option<i32>,
    /// Artist roles (main-artist, composer, etc.).
    pub roles: Option<Vec<String>>,
    /// Associated albums.
    pub albums: Option<ItemSearchResult<Box<Album>>>,
}

/// API response wrapper for artist search (`/artist/search`).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct ArtistSearchResponse {
    /// Artist search results.
    pub artists: ItemSearchResult<Box<Artist>>,
}

/// Generic paginated result container.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ItemSearchResult<T> {
    /// Result items.
    pub items: Option<Vec<T>>,
    /// Total matching items.
    pub total: Option<i32>,
    /// Items per page.
    pub limit: Option<i32>,
    /// Current page offset.
    pub offset: Option<i32>,
}

/// A curated list of tracks.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Playlist {
    /// Unique playlist identifier (may be a string or number from the API).
    #[serde(default, deserialize_with = "deserialize_flexible_string_id")]
    pub id: Option<String>,
    /// Playlist name.
    pub name: Option<String>,
    /// Description text.
    pub description: Option<String>,
    /// Number of tracks.
    pub tracks_count: Option<i32>,
    /// Total duration in seconds.
    pub duration: Option<i32>,
    /// Public visibility.
    pub is_public: Option<bool>,
    /// Playlist creator (full user object, from `/playlist/get`).
    pub creator: Option<User>,
    /// Playlist owner (simpler object with name, from search endpoints).
    pub owner: Option<PlaylistOwner>,
    /// Playlist cover art (always `null` for playlists; use `images`/`image_rectangle` instead).
    pub image: Option<Image>,
    /// Dedicated playlist rectangle banner image URL.
    pub image_rectangle: Option<Vec<String>>,
    /// Smaller playlist rectangle banner image URL.
    pub image_rectangle_mini: Option<Vec<String>>,
    /// Array of 50px track cover URLs (used for collage display).
    pub images: Option<Vec<String>>,
    /// Array of 150px track cover URLs.
    pub images150: Option<Vec<String>>,
    /// Array of 300px track cover URLs.
    pub images300: Option<Vec<String>>,
    /// Contained tracks.
    pub tracks: Option<ItemSearchResult<Box<Track>>>,
    /// Creation timestamp.
    pub created_at: Option<i64>,
    /// Last update timestamp.
    pub updated_at: Option<i64>,
}

impl Playlist {
    /// Returns the display name of the playlist creator/owner, checking
    /// `creator.display_name` first (from `/playlist/get`), then `owner.name`
    /// (from search endpoints).
    ///
    /// # Returns
    ///
    /// `Some(&str)` with the display name if available, `None` otherwise.
    #[must_use]
    pub fn creator_name(&self) -> Option<&str> {
        self.creator
            .as_ref()
            .and_then(|c| c.display_name.as_deref())
            .or_else(|| {
                let o = self.owner.as_ref()?;
                o.name.as_deref()
            })
    }

    /// Returns the best available cover image URL for the playlist.
    ///
    /// For detail views (`large = true`) prefers the dedicated rectangle banner
    /// or the largest track cover. For thumbnails (`large = false`) prefers
    /// the mini rectangle or a medium track cover.
    ///
    /// # Arguments
    ///
    /// * `large` - If `true`, prefers larger image sizes suitable for detail views.
    ///
    /// # Returns
    ///
    /// An optional image URL string.
    #[must_use]
    pub fn best_image_url(&self, large: bool) -> Option<String> {
        if large {
            first_img_url(self.image_rectangle.as_ref())
                .or_else(|| first_img_url(self.images300.as_ref()))
                .or_else(|| first_img_url(self.images150.as_ref()))
                .or_else(|| first_img_url(self.images.as_ref()))
        } else {
            first_img_url(self.image_rectangle_mini.as_ref())
                .or_else(|| first_img_url(self.image_rectangle.as_ref()))
                .or_else(|| first_img_url(self.images150.as_ref()))
                .or_else(|| first_img_url(self.images.as_ref()))
        }
    }
}

/// Playlist owner (returned by search endpoints).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PlaylistOwner {
    /// Owner user ID.
    pub id: Option<i32>,
    /// Owner display name.
    pub name: Option<String>,
}

/// API response wrapper for playlist search (`/playlist/search`).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PlaylistSearchResponse {
    /// Playlist search results.
    pub playlists: ItemSearchResult<Box<Playlist>>,
}

/// Grouped search results across content types.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct SearchResult {
    /// Matching albums.
    pub albums: Option<ItemSearchResult<Box<Album>>>,
    /// Matching artists.
    pub artists: Option<ItemSearchResult<Box<Artist>>>,
    /// Matching tracks.
    pub tracks: Option<ItemSearchResult<Box<Track>>>,
    /// Matching playlists.
    pub playlists: Option<ItemSearchResult<Box<Playlist>>>,
}

/// User subscription details.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Subscription {
    /// Subscription ID.
    pub id: Option<i32>,
    /// Subscription plan name.
    pub offer: Option<String>,
    /// Subscription start date.
    pub start_date: Option<String>,
    /// Subscription end date.
    pub end_date: Option<String>,
    /// Subscription status.
    pub status: Option<String>,
    /// Whether the subscription is active.
    pub is_active: Option<bool>,
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

/// API response wrapper for track search (`/track/search`).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct TrackSearchResponse {
    /// Track search results.
    pub tracks: ItemSearchResult<Box<Track>>,
}

/// A Qobuz user.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct User {
    /// User ID.
    pub id: Option<i32>,
    /// User credentials/capabilities.
    pub credential: Option<Credential>,
    /// Subscription details.
    pub subscription: Option<Subscription>,
    /// Display name.
    pub display_name: Option<String>,
}

/// Collection of user's favorited items.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct UserFavorites {
    /// Favorite albums.
    pub albums: Option<ItemSearchResult<Box<Album>>>,
    /// Favorite artists.
    pub artists: Option<ItemSearchResult<Box<Artist>>>,
    /// Favorite tracks.
    pub tracks: Option<ItemSearchResult<Box<Track>>>,
    /// Favorite article IDs.
    pub article_ids: Option<Vec<i32>>,
    /// Favorite artist IDs.
    pub artist_ids: Option<Vec<i32>>,
    /// Favorite album IDs.
    pub album_ids: Option<Vec<i32>>,
    /// Favorite track IDs.
    pub track_ids: Option<Vec<i32>>,
}

/// Extracts the first URL from an optional vector of image URLs.
///
/// # Arguments
///
/// * `v` - Optional vector of image URLs
///
/// # Returns
///
/// `Some(String)` with the first image URL if the vector is non-empty, `None` otherwise.
fn first_img_url(v: Option<&Vec<String>>) -> Option<String> {
    v?.first().cloned()
}
