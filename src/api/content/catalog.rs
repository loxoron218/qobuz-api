//! Catalog search: searches all content types simultaneously.

use tokio::try_join;

use crate::{
    api::{
        content::{
            albums::search_albums, artists::search_artists, playlists::search_playlists,
            tracks::search_tracks,
        },
        service::QobuzApiService,
    },
    errors::QobuzApiError,
    models::search::SearchResult,
};

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
        test_support::{MockServer, make_service},
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
