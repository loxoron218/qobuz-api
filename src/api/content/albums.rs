//! Album search and browse operations.

use crate::{
    api::{
        content::{
            get_by_id,
            records::{Album, AlbumSearchResponse, ItemSearchResult},
            search,
        },
        service_state::QobuzApiService,
    },
    errors::QobuzApiError,
};

/// Searches for albums matching the query.
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
/// A paginated `ItemSearchResult` containing matching albums.
///
/// # Errors
///
/// Returns a `QobuzApiError` if not authenticated or the API request fails.
pub async fn search_albums(
    service: &QobuzApiService,
    query: &str,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<ItemSearchResult<Box<Album>>, QobuzApiError> {
    let resp: AlbumSearchResponse = search(service, "/album/search", query, limit, offset).await?;
    Ok(resp.albums)
}

/// Retrieves album details by ID.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `album_id` - Album identifier
/// * `extra` - Optional extra fields to include (e.g., `"track_ids"`)
///
/// # Returns
///
/// The album details.
///
/// # Errors
///
/// Returns a `QobuzApiError` if not authenticated or the API request fails.
pub async fn get_album(
    service: &QobuzApiService,
    album_id: &str,
    extra: Option<&str>,
) -> Result<Album, QobuzApiError> {
    get_by_id(service, "/album/get", "album_id", album_id, extra).await
}

#[cfg(test)]
mod tests {
    use {
        anyhow::{Result, anyhow, ensure},
        tokio::runtime::Runtime,
    };

    use crate::{
        api::{
            content::{
                albums::{get_album, search_albums},
                records::Album,
            },
            dev_server::{MockServer, make_service},
        },
        assert_empty_search_test,
    };

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

    #[test]
    fn search_albums_deserializes_results() -> Result<()> {
        let body = r#"{"albums":{"items":[{"id":"123","title":"Test Album"}],"total":1}}"#;
        let server = MockServer::start(200, body)?;
        let service = make_service(&server.base_url())?;
        let rt = Runtime::new()?;
        let result = rt.block_on(search_albums(&service, "Test", Some(5), None))?;
        let items = result.items.ok_or_else(|| anyhow!("no items"))?;
        ensure!(items.len() == 1);
        ensure!(items.first().and_then(|i| i.title.as_deref()) == Some("Test Album"));
        Ok(())
    }

    #[test]
    fn search_albums_empty_results() -> Result<()> {
        assert_empty_search_test!(
            search_albums,
            "Nonexistent",
            r#"{"albums":{"items":[],"total":0}}"#
        );
        Ok(())
    }

    #[test]
    fn search_albums_error_response() -> Result<()> {
        let body = r#"{"status":"error","code":400,"message":"Bad request"}"#;
        let server = MockServer::start(400, body)?;
        let service = make_service(&server.base_url())?;
        let rt = Runtime::new()?;
        let result = rt.block_on(search_albums(&service, "Test", None, None));
        ensure!(result.is_err());
        Ok(())
    }

    #[test]
    fn get_album_by_id() -> Result<()> {
        let body = r#"{"id":"sr6843","title":"Kind of Blue","tracks_count":5}"#;
        let server = MockServer::start(200, body)?;
        let service = make_service(&server.base_url())?;
        let rt = Runtime::new()?;
        let album = rt.block_on(get_album(&service, "sr6843", None))?;
        ensure!(album.title.as_deref() == Some("Kind of Blue"));
        ensure!(album.tracks_count == Some(5));
        Ok(())
    }

    #[test]
    fn get_album_with_extra_param() -> Result<()> {
        let body = r#"{"id":"sr6843","title":"Kind of Blue","track_ids":[1,2,3]}"#;
        let server = MockServer::start(200, body)?;
        let service = make_service(&server.base_url())?;
        let rt = Runtime::new()?;
        let album = rt.block_on(get_album(&service, "sr6843", Some("track_ids")))?;
        let ids = album.track_ids.ok_or_else(|| anyhow!("no track_ids"))?;
        ensure!(ids == vec![1, 2, 3]);
        Ok(())
    }

    #[test]
    fn get_album_not_found() -> Result<()> {
        let body = r#"{"status":"error","code":404,"message":"Album not found"}"#;
        let server = MockServer::start(404, body)?;
        let service = make_service(&server.base_url())?;
        let rt = Runtime::new()?;
        let result = rt.block_on(get_album(&service, "nonexistent", None));
        ensure!(result.is_err());
        Ok(())
    }
}
