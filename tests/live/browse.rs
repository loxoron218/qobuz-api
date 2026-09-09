//! Integration tests for browse/content-detail operations against the live Qobuz API.
//!
//! These tests authenticate against the real API, search for well-known content first,
//! then retrieve detailed metadata by ID — verifying the browse pipeline returns
//! real, non-empty data from Qobuz.
//!
//! **Tests FAIL if credentials are missing or wrong.** There is no silent skip.
//!
//! Setup: copy `.env.example` to `.env` and fill in your credentials, then:
//!
//! Run with `cargo test --test live --features live-tests`.
//!
//! In CI without credentials, run `cargo test` to run only unit tests and the mock integration.

use std::sync::OnceLock;

use crate::{env_var_or, load_env_file};

/// Browse IDs for browse/content-detail tests, initialized once from environment variables.
static BROWSE_IDS: OnceLock<BrowseIds> = OnceLock::new();

/// Test data: IDs and queries configurable via `.env`.
#[derive(Debug)]
pub struct BrowseIds {
    /// Album query string.
    pub album: String,
    /// Artist query string.
    pub artist: String,
    /// Track query string.
    pub track: String,
    /// Playlist query string.
    pub playlist: String,
    /// Release list artist query string.
    pub release_list_artist: String,
}

impl BrowseIds {
    /// Creates IDs from environment variables.
    ///
    /// # Returns
    ///
    /// A new `BrowseIds` instance populated from environment.
    fn from_env() -> Self {
        let env = load_env_file();

        Self {
            album: env_var_or(env, "TEST_BROWSE_ALBUM_QUERY", "Kind of Blue Miles Davis"),
            artist: env_var_or(env, "TEST_BROWSE_ARTIST_QUERY", "Miles Davis"),
            track: env_var_or(env, "TEST_BROWSE_TRACK_QUERY", "So What Miles Davis"),
            playlist: env_var_or(env, "TEST_BROWSE_PLAYLIST_QUERY", "jazz classics"),
            release_list_artist: env_var_or(env, "TEST_BROWSE_RELEASE_LIST_QUERY", "John Coltrane"),
        }
    }
}

/// Returns the browse IDs from environment variables.
///
/// # Returns
///
/// A static reference to the browse IDs.
pub fn get_browse_ids() -> &'static BrowseIds {
    BROWSE_IDS.get_or_init(BrowseIds::from_env)
}

#[cfg(test)]
mod tests {
    use crate::{
        acquisition::query::{get_album_by_query, get_artist_by_query},
        browse::get_browse_ids,
        create_authenticated_service, init_logging, test_support_imports,
    };

    test_support_imports!();

    #[test]
    fn init() {
        init_logging();
    }

    #[test]
    fn live_get_album_returns_real_data() -> Result<()> {
        let ids = get_browse_ids();
        let service = create_authenticated_service()?;

        let album = get_album_by_query(&service, &ids.album, None)?;
        let album_id = album
            .id
            .as_deref()
            .ok_or_else(|| anyhow!("album missing ID"))?;

        info!("Browsing album ID: {album_id}");

        let album: Album = service.get_album(album_id, None)?;

        ensure!(
            album.title.is_some(),
            "get_album should return a title, got: {album:?}"
        );
        ensure!(album.id.is_some(), "get_album should return an ID");

        info!(
            "Album: {} (tracks: {})",
            album.title.as_deref().unwrap_or("?"),
            album
                .tracks_count
                .map_or_else(|| "?".to_string(), |c| c.to_string())
        );
        Ok(())
    }

    #[test]
    fn live_get_album_with_extra_track_ids() -> Result<()> {
        let ids = get_browse_ids();
        let service = create_authenticated_service()?;

        let album: Album = get_album_by_query(&service, &ids.album, Some("track_ids"))?;

        ensure!(
            album.track_ids.is_some(),
            "get_album with extra=track_ids should return track_ids, got: {album:?}"
        );

        let track_ids = album
            .track_ids
            .as_ref()
            .ok_or_else(|| anyhow!("track_ids should be present"))?;
        ensure!(
            !track_ids.is_empty(),
            "album should have at least one track ID"
        );

        info!(
            "Album '{}' has {} tracks: {:?}",
            album.title.as_deref().unwrap_or("?"),
            track_ids.len(),
            track_ids.iter().take(5).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn live_get_artist_returns_real_data() -> Result<()> {
        let ids = get_browse_ids();
        let service = create_authenticated_service()?;

        let artist = get_artist_by_query(&service, &ids.artist, None)?;
        let artist_id = artist.id.ok_or_else(|| anyhow!("artist missing ID"))?;

        info!("Browsing artist ID: {artist_id}");

        let artist: Artist = service.get_artist(artist_id, None)?;

        ensure!(
            artist.name.is_some(),
            "get_artist should return a name, got: {artist:?}"
        );

        info!(
            "Artist: {} (albums: {})",
            artist.name.as_deref().unwrap_or("?"),
            artist
                .albums_count
                .map_or_else(|| "?".to_string(), |c| c.to_string())
        );
        Ok(())
    }

    #[test]
    fn live_get_track_returns_real_data() -> Result<()> {
        let ids = get_browse_ids();
        let service = create_authenticated_service()?;

        let search = service.search_tracks(&ids.track, Some(1), None)?;
        let items = search
            .items
            .ok_or_else(|| anyhow!("search_tracks returned no items for '{}'", ids.track))?;

        let first = items
            .first()
            .ok_or_else(|| anyhow!("empty track results for '{}'", ids.track))?;

        let track_id = first.id.ok_or_else(|| anyhow!("track missing ID"))?;

        info!("Browsing track ID: {track_id}");

        let track: Track = service.get_track(track_id)?;

        ensure!(
            track.title.is_some(),
            "get_track should return a title, got: {track:?}"
        );
        ensure!(track.duration.is_some(), "get_track should return duration");

        info!(
            "Track: {} (duration: {}s, album: {})",
            track.title.as_deref().unwrap_or("?"),
            track
                .duration
                .map_or_else(|| "?".to_string(), |d| d.to_string()),
            track
                .album
                .as_ref()
                .and_then(|a| a.title.as_deref())
                .unwrap_or("?")
        );
        Ok(())
    }

    #[test]
    fn live_get_track_has_album_and_artist_metadata() -> Result<()> {
        let ids = get_browse_ids();
        let service = create_authenticated_service()?;

        let search = service.search_tracks(&ids.track, Some(5), None)?;
        let items = search
            .items
            .ok_or_else(|| anyhow!("search_tracks returned no items"))?;

        let track_with_album = items
            .iter()
            .find(|t| t.album.is_some() && t.performer.is_some())
            .ok_or_else(|| anyhow!("no track with album and performer metadata found"))?;

        let track_id = track_with_album
            .id
            .ok_or_else(|| anyhow!("track missing ID"))?;

        let track: Track = service.get_track(track_id)?;

        ensure!(track.album.is_some(), "track should have album metadata");
        ensure!(
            track.performer.is_some(),
            "track should have performer metadata"
        );

        let album_title = track
            .album
            .as_ref()
            .and_then(|a| a.title.as_deref())
            .unwrap_or("?");
        let performer = track
            .performer
            .as_ref()
            .and_then(|p| p.name.as_deref())
            .unwrap_or("?");

        info!(
            "Track: {} by {performer} from '{album_title}'",
            track.title.as_deref().unwrap_or("?")
        );
        Ok(())
    }

    #[test]
    fn live_get_playlist_returns_real_data() -> Result<()> {
        let ids = get_browse_ids();
        let service = create_authenticated_service()?;

        let search = service.search_playlists(&ids.playlist, Some(1), None)?;
        let items = search
            .items
            .as_ref()
            .ok_or_else(|| anyhow!("search_playlists returned no items for '{}'", ids.playlist))?;

        let first = items
            .first()
            .ok_or_else(|| anyhow!("empty playlist results for '{}'", ids.playlist))?;

        let playlist_id = first
            .id
            .as_deref()
            .ok_or_else(|| anyhow!("playlist missing ID"))?;

        info!("Browsing playlist ID: {playlist_id}");

        let playlist: Playlist = service.get_playlist(playlist_id, None)?;

        ensure!(
            playlist.name.is_some(),
            "get_playlist should return a name, got: {playlist:?}"
        );

        info!(
            "Playlist: {} (tracks: {})",
            playlist.name.as_deref().unwrap_or("?"),
            playlist
                .tracks_count
                .map_or_else(|| "?".to_string(), |c| c.to_string())
        );
        Ok(())
    }

    #[test]
    fn live_get_release_list_returns_albums() -> Result<()> {
        let ids = get_browse_ids();
        let service = create_authenticated_service()?;

        let search = service.search_artists(&ids.release_list_artist, Some(1), None)?;
        let items = search.items.ok_or_else(|| {
            anyhow!(
                "search_artists returned no items for '{}'",
                ids.release_list_artist
            )
        })?;

        let first = items
            .first()
            .ok_or_else(|| anyhow!("empty artist results"))?;

        let artist_id = first.id.ok_or_else(|| anyhow!("artist missing ID"))?;

        info!("Getting release list for artist ID: {artist_id}");

        let releases = service.get_release_list(artist_id, Some(5), None)?;
        let release_items = releases
            .items
            .ok_or_else(|| anyhow!("get_release_list returned no items"))?;

        ensure!(
            !release_items.is_empty(),
            "get_release_list should return at least one album"
        );

        ensure!(
            release_items.iter().any(|a| a.title.is_some()),
            "At least one release should have a title"
        );

        info!("Release list ({} albums):", release_items.len());
        for (i, album) in release_items.iter().take(5).enumerate() {
            info!(
                "  {}. {}",
                i.saturating_add(1),
                album.title.as_deref().unwrap_or("?")
            );
        }
        Ok(())
    }

    #[test]
    fn live_browse_nonexistent_album_returns_error() -> Result<()> {
        let service = create_authenticated_service()?;

        let result = service.get_album("999999999999", None);
        ensure!(
            result.is_err(),
            "get_album with nonexistent ID should return error, got: {result:?}"
        );
        Ok(())
    }

    #[test]
    fn live_browse_nonexistent_track_returns_error() -> Result<()> {
        let service = create_authenticated_service()?;

        let result = service.get_track(999_999_999);
        ensure!(
            result.is_err(),
            "get_track with nonexistent ID should return error, got: {result:?}"
        );
        Ok(())
    }
}
