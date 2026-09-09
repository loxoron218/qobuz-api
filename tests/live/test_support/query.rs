//! Query helpers for finding albums, artists, and tracks by search query.

use anyhow::{Error, Result, anyhow};

use qobuz_api::{
    api::service::QobuzApiService,
    models::{album::Album, artist::Artist},
};

/// Finds a track ID by query string.
///
/// # Arguments
///
/// * `service` - The Qobuz API service
/// * `query` - Search query string
///
/// # Returns
///
/// The track ID if found.
///
/// # Errors
///
/// Returns an error if the search fails or no track ID is found.
pub fn find_track_id(service: &QobuzApiService, query: &str) -> Result<i32> {
    let search = service.search_tracks(query, Some(1), None)?;
    let items = search
        .items
        .ok_or_else(|| anyhow!("search_tracks returned no items for '{query}'"))?;

    let first = items
        .first()
        .ok_or_else(|| anyhow!("empty track results for '{query}'"))?;

    first.id.ok_or_else(|| anyhow!("track missing ID"))
}

/// Finds an album ID by query string.
///
/// # Arguments
///
/// * `service` - The Qobuz API service
/// * `query` - Search query string
///
/// # Returns
///
/// The album ID if found.
///
/// # Errors
///
/// Returns an error if the search fails or no album ID is found.
pub fn find_album_id(service: &QobuzApiService, query: &str) -> Result<String> {
    let search = service.search_albums(query, Some(1), None)?;
    let items = search
        .items
        .ok_or_else(|| anyhow!("search_albums returned no items for '{query}'"))?;

    let first = items
        .first()
        .ok_or_else(|| anyhow!("empty album results for '{query}'"))?;

    first
        .id
        .as_ref()
        .ok_or_else(|| anyhow!("album missing ID"))
        .map(String::from)
}

/// Finds an artist ID by query string.
///
/// # Arguments
///
/// * `service` - The Qobuz API service
/// * `query` - Search query string
///
/// # Returns
///
/// The artist ID if found.
///
/// # Errors
///
/// Returns an error if the search fails or no artist ID is found.
pub fn find_artist_id(service: &QobuzApiService, query: &str) -> Result<i32> {
    let search = service.search_artists(query, Some(1), None)?;
    let items = search
        .items
        .ok_or_else(|| anyhow!("search_artists returned no items for '{query}'"))?;

    let first = items
        .first()
        .ok_or_else(|| anyhow!("empty artist results for '{query}'"))?;

    first.id.ok_or_else(|| anyhow!("artist missing ID"))
}

/// Gets an album by query string.
///
/// # Arguments
///
/// * `service` - The Qobuz API service
/// * `query` - Search query string
/// * `extra` - Optional extra fields to include
///
/// # Returns
///
/// The album if found.
///
/// # Errors
///
/// Returns an error if the search fails or the album cannot be fetched.
pub fn get_album_by_query(
    service: &QobuzApiService,
    query: &str,
    extra: Option<&str>,
) -> Result<Album> {
    let album_id = find_album_id(service, query)?;
    service.get_album(&album_id, extra).map_err(Error::from)
}

/// Gets an artist by query string.
///
/// # Arguments
///
/// * `service` - The Qobuz API service
/// * `query` - Search query string
/// * `extra` - Optional extra fields to include
///
/// # Returns
///
/// The artist if found.
///
/// # Errors
///
/// Returns an error if the search fails or the artist cannot be fetched.
pub fn get_artist_by_query(
    service: &QobuzApiService,
    query: &str,
    extra: Option<&str>,
) -> Result<Artist> {
    let artist_id = find_artist_id(service, query)?;
    service.get_artist(artist_id, extra).map_err(Error::from)
}
