//! Playlist search and browse operations.

use crate::{
    api::{
        content::{get_by_id, search},
        service::QobuzApiService,
    },
    errors::QobuzApiError,
    models::{
        playlist::Playlist,
        search::{ItemSearchResult, PlaylistSearchResponse},
    },
};

/// Searches for playlists matching the query.
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
/// A paginated `ItemSearchResult` containing matching playlists.
///
/// # Errors
///
/// Returns a `QobuzApiError` if not authenticated or the API request fails.
pub async fn search_playlists(
    service: &QobuzApiService,
    query: &str,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<ItemSearchResult<Box<Playlist>>, QobuzApiError> {
    let resp: PlaylistSearchResponse =
        search(service, "/playlist/search", query, limit, offset).await?;
    Ok(resp.playlists)
}

/// Retrieves playlist details by ID.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `playlist_id` - Playlist identifier
/// * `extra` - Optional extra fields to include
///
/// # Returns
///
/// The playlist details.
///
/// # Errors
///
/// Returns a `QobuzApiError` if not authenticated or the API request fails.
pub async fn get_playlist(
    service: &QobuzApiService,
    playlist_id: &str,
    extra: Option<&str>,
) -> Result<Playlist, QobuzApiError> {
    get_by_id(service, "/playlist/get", "playlist_id", playlist_id, extra).await
}

#[cfg(test)]
mod tests {
    use {
        anyhow::{Result, anyhow, ensure},
        tokio::runtime::Runtime,
    };

    use crate::{
        api::content::playlists::{get_playlist, search_playlists},
        assert_empty_search_test, setup_test,
    };

    /// Tests search playlists deserializes results.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
    #[test]
    fn search_playlists_deserializes_results() -> Result<()> {
        setup_test!(
            200,
            r#"{"playlists":{"items":[{"id":"pl1","name":"Jazz Mix"}],"total":1}}"#,
            server,
            service,
            rt
        );
        let result = rt.block_on(search_playlists(&service, "Jazz", Some(5), None))?;
        let items = result.items.ok_or_else(|| anyhow!("no items"))?;
        ensure!(items.len() == 1);
        ensure!(items.first().and_then(|i| i.name.as_deref()) == Some("Jazz Mix"));
        Ok(())
    }

    /// Tests search playlists empty results.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
    #[test]
    fn search_playlists_empty_results() -> Result<()> {
        assert_empty_search_test!(
            search_playlists,
            "Nothing",
            r#"{"playlists":{"items":[],"total":0}}"#
        );
        Ok(())
    }

    /// Tests get playlist by id.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
    #[test]
    fn get_playlist_by_id() -> Result<()> {
        setup_test!(
            200,
            r#"{"id":"pl42","name":"My Playlist","tracks_count":10}"#,
            server,
            service,
            rt
        );
        let playlist = rt.block_on(get_playlist(&service, "pl42", None))?;
        ensure!(playlist.name.as_deref() == Some("My Playlist"));
        let playlist = rt.block_on(get_playlist(&service, "pl42", Some("tracks")))?;
        ensure!(playlist.name.as_deref() == Some("My Playlist"));
        ensure!(playlist.tracks_count == Some(10));
        Ok(())
    }

    /// Tests search playlists error response.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
    #[test]
    fn search_playlists_error_response() -> Result<()> {
        setup_test!(
            401,
            r#"{"status":"error","code":401,"message":"Unauthorized"}"#,
            server,
            service,
            rt
        );
        let result = rt.block_on(search_playlists(&service, "fail", None, None));
        ensure!(result.is_err());
        Ok(())
    }

    /// Tests get playlist not found.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
    #[test]
    fn get_playlist_not_found() -> Result<()> {
        setup_test!(
            404,
            r#"{"status":"error","code":404,"message":"Playlist not found"}"#,
            server,
            service,
            rt
        );
        let result = rt.block_on(get_playlist(&service, "nonexistent", None));
        ensure!(result.is_err());
        Ok(())
    }
}
