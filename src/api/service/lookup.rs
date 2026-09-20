//! Service delegates for detail lookup operations.

use tokio::runtime::Runtime;

use crate::api::{
    content::{
        albums::{Album, get_album},
        artists::{Artist, get_artist, get_release_list},
        catalog::ItemSearchResult,
        playlists::{Playlist, get_playlist},
        tracks::{Track, get_track},
    },
    service::QobuzApiService,
};

impl QobuzApiService {
    delegate!(
        /// Retrieves album details by ID.
        ///
        /// # Arguments
        ///
        /// * `album_id` - Album identifier
        /// * `extra` - Optional extra fields to include (e.g., `"track_ids"`)
        ///
        /// # Returns
        ///
        /// The album details.
        pub fn get_album(album_id: &str, extra: Option<&str>) -> Album = get_album
    );

    delegate!(
        /// Retrieves artist details by ID.
        ///
        /// # Arguments
        ///
        /// * `artist_id` - Artist identifier
        /// * `extra` - Optional extra fields to include
        ///
        /// # Returns
        ///
        /// The artist details.
        pub fn get_artist(artist_id: i32, extra: Option<&str>) -> Artist = get_artist
    );

    delegate!(
        /// Retrieves track details by ID.
        ///
        /// # Arguments
        ///
        /// * `track_id` - Track identifier
        ///
        /// # Returns
        ///
        /// The track details.
        pub fn get_track(track_id: i32) -> Track = get_track
    );

    delegate!(
        /// Retrieves playlist details by ID.
        ///
        /// # Arguments
        ///
        /// * `playlist_id` - Playlist identifier
        /// * `extra` - Optional extra fields to include
        ///
        /// # Returns
        ///
        /// The playlist details.
        pub fn get_playlist(playlist_id: &str, extra: Option<&str>) -> Playlist = get_playlist
    );

    delegate!(
        /// Retrieves an artist's release list (discography).
        ///
        /// # Arguments
        ///
        /// * `artist_id` - Artist identifier
        /// * `limit` - Maximum number of releases to return
        /// * `offset` - Pagination offset
        ///
        /// # Returns
        ///
        /// A paginated `ItemSearchResult` containing the artist's albums.
        pub fn get_release_list(artist_id: i32, limit: Option<i32>, offset: Option<i32>) -> ItemSearchResult<Box<Album>> = get_release_list
    );
}

#[cfg(test)]
mod tests {
    use anyhow::{Result, anyhow, ensure};

    use crate::api::fixture::{MockServer, make_service};

    #[test]
    fn get_album_delegate_returns_details() -> Result<()> {
        let server = MockServer::start(
            200,
            r#"{"id":"sr9999","title":"Blue Train","tracks_count":11}"#,
        )?;
        let service = make_service(&server.base_url())?;
        let album = service.get_album("sr9999", None)?;
        ensure!(album.title.as_deref() == Some("Blue Train"));
        ensure!(album.tracks_count == Some(11));
        Ok(())
    }

    #[test]
    fn get_artist_delegate_returns_details() -> Result<()> {
        let server = MockServer::start(200, r#"{"id":99,"name":"Miles Davis"}"#)?;
        let service = make_service(&server.base_url())?;
        let artist = service.get_artist(99, None)?;
        ensure!(artist.name.as_deref() == Some("Miles Davis"));
        Ok(())
    }

    #[test]
    fn get_track_delegate_returns_details() -> Result<()> {
        let server = MockServer::start(200, r#"{"id":77,"title":"So What"}"#)?;
        let service = make_service(&server.base_url())?;
        let track = service.get_track(77)?;
        ensure!(track.title.as_deref() == Some("So What"));
        Ok(())
    }

    #[test]
    fn get_playlist_delegate_returns_details() -> Result<()> {
        let server =
            MockServer::start(200, r#"{"id":"pl7","name":"Late Night","tracks_count":3}"#)?;
        let service = make_service(&server.base_url())?;
        let playlist = service.get_playlist("pl7", Some("tracks"))?;
        ensure!(playlist.name.as_deref() == Some("Late Night"));
        ensure!(playlist.tracks_count == Some(3));
        Ok(())
    }

    #[test]
    fn get_release_list_delegate_returns_albums() -> Result<()> {
        let server = MockServer::start(
            200,
            r#"{"items":[{"id":"b2","title":"Kind of Blue"}],"total":1}"#,
        )?;
        let service = make_service(&server.base_url())?;
        let result = service.get_release_list(99, Some(10), None)?;
        ensure!(result.total == Some(1), "expected total 1");
        let items = result.items.ok_or_else(|| anyhow!("no items"))?;
        ensure!(items.len() == 1, "expected one release");
        ensure!(items.first().and_then(|i| i.title.as_deref()) == Some("Kind of Blue"));
        Ok(())
    }
}
