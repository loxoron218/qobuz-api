//! Search result data models.

use serde::{Deserialize, Serialize};

use crate::models::{album::Album, artist::Artist, playlist::Playlist, track::Track};

/// API response wrapper for album search (`/album/search`).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct AlbumSearchResponse {
    /// Album search results.
    pub albums: ItemSearchResult<Box<Album>>,
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

/// API response wrapper for track search (`/track/search`).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct TrackSearchResponse {
    /// Track search results.
    pub tracks: ItemSearchResult<Box<Track>>,
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
