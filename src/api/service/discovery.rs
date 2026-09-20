//! Service delegates for content search operations.

use tokio::runtime::Runtime;

use crate::api::{
    content::{
        albums::{Album, search_albums},
        artists::{Artist, search_artists},
        catalog::{ItemSearchResult, SearchResult, search_catalog},
        playlists::{Playlist, search_playlists},
        tracks::{Track, search_tracks},
    },
    service::QobuzApiService,
};

impl QobuzApiService {
    delegate!(
        /// Searches all content types (albums, artists, tracks, playlists).
        ///
        /// Returns grouped results for each content type.
        ///
        /// # Arguments
        ///
        /// * `query` - Search query string
        /// * `limit` - Maximum number of results per content type
        /// * `offset` - Pagination offset
        ///
        /// # Returns
        ///
        /// A `SearchResult` with grouped results for each content type.
        pub fn search_catalog(query: &str, limit: Option<i32>, offset: Option<i32>) -> SearchResult = search_catalog
    );

    delegate!(
        /// Searches for albums matching the query.
        ///
        /// # Arguments
        ///
        /// * `query` - Search query string
        /// * `limit` - Maximum number of results to return
        /// * `offset` - Pagination offset
        ///
        /// # Returns
        ///
        /// A paginated `ItemSearchResult` containing matching albums.
        pub fn search_albums(query: &str, limit: Option<i32>, offset: Option<i32>) -> ItemSearchResult<Box<Album>> = search_albums
    );

    delegate!(
        /// Searches for artists matching the query.
        ///
        /// # Arguments
        ///
        /// * `query` - Search query string
        /// * `limit` - Maximum number of results to return
        /// * `offset` - Pagination offset
        ///
        /// # Returns
        ///
        /// A paginated `ItemSearchResult` containing matching artists.
        pub fn search_artists(query: &str, limit: Option<i32>, offset: Option<i32>) -> ItemSearchResult<Box<Artist>> = search_artists
    );

    delegate!(
        /// Searches for tracks matching the query.
        ///
        /// # Arguments
        ///
        /// * `query` - Search query string
        /// * `limit` - Maximum number of results to return
        /// * `offset` - Pagination offset
        ///
        /// # Returns
        ///
        /// A paginated `ItemSearchResult` containing matching tracks.
        pub fn search_tracks(query: &str, limit: Option<i32>, offset: Option<i32>) -> ItemSearchResult<Box<Track>> = search_tracks
    );

    delegate!(
        /// Searches for playlists matching the query.
        ///
        /// # Arguments
        ///
        /// * `query` - Search query string
        /// * `limit` - Maximum number of results to return
        /// * `offset` - Pagination offset
        ///
        /// # Returns
        ///
        /// A paginated `ItemSearchResult` containing matching playlists.
        pub fn search_playlists(query: &str, limit: Option<i32>, offset: Option<i32>) -> ItemSearchResult<Box<Playlist>> = search_playlists
    );
}

#[cfg(test)]
mod tests {
    use anyhow::{Result, anyhow, ensure};

    use crate::api::fixture::{MockServer, make_service};

    #[test]
    fn search_catalog_delegate_returns_all_groups() -> Result<()> {
        let body = r#"{"albums":{"items":[{"id":"s1","title":"Blue"}],"total":1},"artists":{"items":[],"total":0},"tracks":{"items":[],"total":0},"playlists":{"items":[],"total":0}}"#;
        let server = MockServer::start_with_max_requests(200, body, 16)?;
        let service = make_service(&server.base_url())?;
        let result = service.search_catalog("blue", Some(3), None)?;
        let albums = result.albums.ok_or_else(|| anyhow!("no albums"))?;
        ensure!(albums.total == Some(1), "expected total 1");
        let items = albums.items.ok_or_else(|| anyhow!("no items"))?;
        ensure!(items.first().and_then(|a| a.title.as_deref()) == Some("Blue"));
        ensure!(
            result.artists.is_some() && result.tracks.is_some() && result.playlists.is_some(),
            "expected all result groups"
        );
        Ok(())
    }

    #[test]
    fn search_albums_delegate_finds_matches() -> Result<()> {
        let server = MockServer::start(
            200,
            r#"{"albums":{"items":[{"id":"x1","title":"Blue Train"}],"total":1}}"#,
        )?;
        let service = make_service(&server.base_url())?;
        let result = service.search_albums("blue train", Some(5), None)?;
        ensure!(result.total == Some(1), "expected total 1");
        let items = result.items.ok_or_else(|| anyhow!("no items"))?;
        ensure!(items.len() == 1, "expected one match");
        ensure!(items.first().and_then(|a| a.title.as_deref()) == Some("Blue Train"));
        Ok(())
    }

    #[test]
    fn search_artists_delegate_finds_matches() -> Result<()> {
        let server = MockServer::start(
            200,
            r#"{"artists":{"items":[{"id":7,"name":"Miles Davis"}],"total":1}}"#,
        )?;
        let service = make_service(&server.base_url())?;
        let result = service.search_artists("miles", Some(10), None)?;
        let items = result.items.ok_or_else(|| anyhow!("no items"))?;
        ensure!(items.len() == 1, "expected one match");
        ensure!(items.first().and_then(|a| a.name.as_deref()) == Some("Miles Davis"));
        Ok(())
    }

    #[test]
    fn search_tracks_delegate_finds_matches() -> Result<()> {
        let server = MockServer::start(
            200,
            r#"{"tracks":{"items":[{"id":9,"title":"So What"}],"total":1}}"#,
        )?;
        let service = make_service(&server.base_url())?;
        let result = service.search_tracks("so what", None, None)?;
        ensure!(result.total == Some(1), "expected total 1");
        let items = result.items.ok_or_else(|| anyhow!("no items"))?;
        ensure!(items.first().and_then(|t| t.title.as_deref()) == Some("So What"));
        Ok(())
    }

    #[test]
    fn search_playlists_delegate_finds_matches() -> Result<()> {
        let server = MockServer::start(
            200,
            r#"{"playlists":{"items":[{"id":"p9","name":"Late Night"}],"total":1}}"#,
        )?;
        let service = make_service(&server.base_url())?;
        let result = service.search_playlists("late night", Some(5), Some(0))?;
        let items = result.items.ok_or_else(|| anyhow!("no items"))?;
        ensure!(!items.is_empty(), "expected matches");
        ensure!(items.first().and_then(|p| p.name.as_deref()) == Some("Late Night"));
        Ok(())
    }
}
