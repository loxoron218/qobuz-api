//! Catalog search: searches all content types simultaneously.

use {
    serde::{Deserialize, Serialize},
    tokio::try_join,
};

use crate::{
    api::{
        content::{
            albums::{Album, search_albums},
            artists::{Artist, search_artists},
            playlists::{Playlist, search_playlists},
            tracks::{Track, search_tracks},
        },
        service::QobuzApiService,
    },
    errors::QobuzApiError,
};

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

/// Searches all content types (albums, artists, tracks, playlists).
///
/// Returns grouped results for each content type.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `query` - Search query string
/// * `limit` - Maximum number of results per content type
/// * `offset` - Pagination offset
///
/// # Returns
///
/// A `SearchResult` with grouped results for each content type.
///
/// # Errors
///
/// Returns a `QobuzApiError` if any of the parallel search requests fails.
pub async fn search_catalog(
    service: &QobuzApiService,
    query: &str,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<SearchResult, QobuzApiError> {
    let (albums, artists, tracks, playlists) = try_join!(
        search_albums(service, query, limit, offset),
        search_artists(service, query, limit, offset),
        search_tracks(service, query, limit, offset),
        search_playlists(service, query, limit, offset),
    )?;

    Ok(SearchResult {
        albums: Some(albums),
        artists: Some(artists),
        tracks: Some(tracks),
        playlists: Some(playlists),
    })
}

#[cfg(test)]
mod tests {
    use {
        anyhow::{Result, ensure},
        tokio::runtime::Runtime,
    };

    use crate::api::{
        content::catalog::search_catalog,
        fixture::{MockServer, make_service},
    };

    /// Tests search catalog groups all types.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
    #[test]
    fn search_catalog_groups_all_types() -> Result<()> {
        let body = r#"{"albums":{"items":[],"total":0},"artists":{"items":[],"total":0},"tracks":{"items":[],"total":0},"playlists":{"items":[],"total":0}}"#;
        let server = MockServer::start_with_max_requests(200, body, 16)?;
        let service = make_service(&server.base_url())?;
        let rt = Runtime::new()?;
        let result = rt.block_on(search_catalog(&service, "Test", Some(5), None))?;
        ensure!(result.albums.is_some());
        ensure!(result.artists.is_some());
        ensure!(result.tracks.is_some());
        ensure!(result.playlists.is_some());
        Ok(())
    }

    /// Tests search catalog error stops all.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
    #[test]
    fn search_catalog_error_stops_all() -> Result<()> {
        let body = r#"{"status":"error","code":500,"message":"Fail"}"#;
        let server = MockServer::start_with_max_requests(500, body, 16)?;
        let service = make_service(&server.base_url())?;
        let rt = Runtime::new()?;
        let result = rt.block_on(search_catalog(&service, "fail", None, None));
        ensure!(result.is_err());
        Ok(())
    }
}
