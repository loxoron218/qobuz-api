//! Artist search and browse operations.

use crate::{
    api::{
        content::{get_by_id, paginated, search},
        service::QobuzApiService,
    },
    errors::QobuzApiError,
    models::{
        album::Album,
        artist::Artist,
        search::{ArtistSearchResponse, ItemSearchResult},
    },
};
/// Searches for artists matching the query.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `query` - Search query string
/// * `limit` - Maximum number of results to return
/// * `offset` - Pagination offset
///
/// # Returns
///
/// A paginated `ItemSearchResult` containing matching artists.
///
/// # Errors
///
/// Returns a `QobuzApiError` if not authenticated or the API request fails.
pub async fn search_artists(
    service: &QobuzApiService,
    query: &str,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<ItemSearchResult<Box<Artist>>, QobuzApiError> {
    let resp: ArtistSearchResponse =
        search(service, "/artist/search", query, limit, offset).await?;
    Ok(resp.artists)
}

/// Retrieves artist details by ID.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `artist_id` - Artist identifier
/// * `extra` - Optional extra fields to include
///
/// # Returns
///
/// The artist details.
///
/// # Errors
///
/// Returns a `QobuzApiError` if not authenticated or the API request fails.
pub async fn get_artist(
    service: &QobuzApiService,
    artist_id: i32,
    extra: Option<&str>,
) -> Result<Artist, QobuzApiError> {
    get_by_id(service, "/artist/get", "artist_id", artist_id, extra).await
}

/// Retrieves an artist's release list (discography).
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `artist_id` - Artist identifier
/// * `limit` - Maximum number of releases to return
/// * `offset` - Pagination offset
///
/// # Returns
///
/// A paginated `ItemSearchResult` containing the artist's albums.
///
/// # Errors
///
/// Returns a `QobuzApiError` if not authenticated or the API request fails.
pub async fn get_release_list(
    service: &QobuzApiService,
    artist_id: i32,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<ItemSearchResult<Box<Album>>, QobuzApiError> {
    paginated(
        service,
        "/artist/getReleasesList",
        "artist_id",
        artist_id,
        limit,
        offset,
    )
    .await
}

#[cfg(test)]
mod tests {
    use {
        anyhow::{Result, anyhow, ensure},
        tokio::runtime::Runtime,
    };

    use crate::{
        api::{
            content::artists::{get_artist, get_release_list, search_artists},
            test_support::{MockServer, make_service},
        },
        assert_empty_search_test,
    };

    #[test]
    fn search_artists_deserializes_results() -> Result<()> {
        let body = r#"{"artists":{"items":[{"id":1,"name":"Miles Davis"}],"total":1}}"#;
        let server = MockServer::start(200, body)?;
        let service = make_service(&server.base_url())?;
        let rt = Runtime::new()?;
        let result = rt.block_on(search_artists(&service, "Miles", Some(5), None))?;
        let items = result.items.ok_or_else(|| anyhow!("no items"))?;
        ensure!(items.len() == 1);
        ensure!(items.first().and_then(|i| i.name.as_deref()) == Some("Miles Davis"));
        Ok(())
    }

    #[test]
    fn search_artists_empty_results() -> Result<()> {
        assert_empty_search_test!(
            search_artists,
            "Nobody",
            r#"{"artists":{"items":[],"total":0}}"#
        );
        Ok(())
    }

    #[test]
    fn get_artist_by_id() -> Result<()> {
        let body = r#"{"id":42,"name":"Coltrane"}"#;
        let server = MockServer::start(200, body)?;
        let service = make_service(&server.base_url())?;
        let rt = Runtime::new()?;
        let artist = rt.block_on(get_artist(&service, 42, None))?;
        ensure!(artist.name.as_deref() == Some("Coltrane"));
        Ok(())
    }

    #[test]
    fn get_artist_error_response() -> Result<()> {
        let body = r#"{"status":"error","code":404,"message":"Not found"}"#;
        let server = MockServer::start(404, body)?;
        let service = make_service(&server.base_url())?;
        let rt = Runtime::new()?;
        let result = rt.block_on(get_artist(&service, 99999, None));
        ensure!(result.is_err());
        Ok(())
    }

    #[test]
    fn get_artist_with_extra() -> Result<()> {
        let body = r#"{"id":42,"name":"Coltrane","albums_count":10}"#;
        let server = MockServer::start(200, body)?;
        let service = make_service(&server.base_url())?;
        let rt = Runtime::new()?;
        let artist = rt.block_on(get_artist(&service, 42, Some("albums")))?;
        ensure!(artist.name.as_deref() == Some("Coltrane"));
        Ok(())
    }

    #[test]
    fn get_release_list_returns_albums() -> Result<()> {
        let body = r#"{"items":[{"id":"a1","title":"A Love Supreme"}],"total":1}"#;
        let server = MockServer::start(200, body)?;
        let service = make_service(&server.base_url())?;
        let rt = Runtime::new()?;
        let result = rt.block_on(get_release_list(&service, 42, Some(5), None))?;
        let items = result.items.ok_or_else(|| anyhow!("no items"))?;
        ensure!(items.len() == 1);
        ensure!(items.first().and_then(|i| i.title.as_deref()) == Some("A Love Supreme"));
        Ok(())
    }
}
