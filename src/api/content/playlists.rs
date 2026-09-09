//! Playlist search and browse operations.

use serde::{Deserialize, Serialize};

use crate::{
    api::{
        auth::User,
        content::{
            albums::Image,
            catalog::{ItemSearchResult, PlaylistSearchResponse},
            get_by_id, search,
            tracks::Track,
        },
        response::deserialize_flexible_string_id,
        service::QobuzApiService,
    },
    errors::QobuzApiError,
};

/// A curated list of tracks.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Playlist {
    /// Unique playlist identifier (may be a string or number from the API).
    #[serde(default, deserialize_with = "deserialize_flexible_string_id")]
    pub id: Option<String>,
    /// Playlist name.
    pub name: Option<String>,
    /// Description text.
    pub description: Option<String>,
    /// Number of tracks.
    pub tracks_count: Option<i32>,
    /// Total duration in seconds.
    pub duration: Option<i32>,
    /// Public visibility.
    pub is_public: Option<bool>,
    /// Playlist creator (full user object, from `/playlist/get`).
    pub creator: Option<User>,
    /// Playlist owner (simpler object with name, from search endpoints).
    pub owner: Option<PlaylistOwner>,
    /// Playlist cover art (always `null` for playlists; use `images`/`image_rectangle` instead).
    pub image: Option<Image>,
    /// Dedicated playlist rectangle banner image URL.
    pub image_rectangle: Option<Vec<String>>,
    /// Smaller playlist rectangle banner image URL.
    pub image_rectangle_mini: Option<Vec<String>>,
    /// Array of 50px track cover URLs (used for collage display).
    pub images: Option<Vec<String>>,
    /// Array of 150px track cover URLs.
    pub images150: Option<Vec<String>>,
    /// Array of 300px track cover URLs.
    pub images300: Option<Vec<String>>,
    /// Contained tracks.
    pub tracks: Option<ItemSearchResult<Box<Track>>>,
    /// Creation timestamp.
    pub created_at: Option<i64>,
    /// Last update timestamp.
    pub updated_at: Option<i64>,
}

impl Playlist {
    /// Returns the display name of the playlist creator/owner, checking
    /// `creator.display_name` first (from `/playlist/get`), then `owner.name`
    /// (from search endpoints).
    ///
    /// # Returns
    ///
    /// `Some(&str)` with the display name if available, `None` otherwise.
    #[must_use]
    pub fn creator_name(&self) -> Option<&str> {
        self.creator
            .as_ref()
            .and_then(|c| c.display_name.as_deref())
            .or_else(|| {
                let o = self.owner.as_ref()?;
                o.name.as_deref()
            })
    }

    /// Returns the best available cover image URL for the playlist.
    ///
    /// For detail views (`large = true`) prefers the dedicated rectangle banner
    /// or the largest track cover. For thumbnails (`large = false`) prefers
    /// the mini rectangle or a medium track cover.
    ///
    /// # Arguments
    ///
    /// * `large` - If `true`, prefers larger image sizes suitable for detail views.
    ///
    /// # Returns
    ///
    /// An optional image URL string.
    #[must_use]
    pub fn best_image_url(&self, large: bool) -> Option<String> {
        if large {
            first_img_url(self.image_rectangle.as_ref())
                .or_else(|| first_img_url(self.images300.as_ref()))
                .or_else(|| first_img_url(self.images150.as_ref()))
                .or_else(|| first_img_url(self.images.as_ref()))
        } else {
            first_img_url(self.image_rectangle_mini.as_ref())
                .or_else(|| first_img_url(self.image_rectangle.as_ref()))
                .or_else(|| first_img_url(self.images150.as_ref()))
                .or_else(|| first_img_url(self.images.as_ref()))
        }
    }
}

/// Playlist owner (returned by search endpoints).
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct PlaylistOwner {
    /// Owner user ID.
    pub id: Option<i32>,
    /// Owner display name.
    pub name: Option<String>,
}

/// Extracts the first URL from an optional vector of image URLs.
///
/// # Returns
///
/// `Some(String)` with the first image URL if the vector is non-empty, `None` otherwise.
fn first_img_url(v: Option<&Vec<String>>) -> Option<String> {
    v?.first().cloned()
}

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
        api::{
            auth::User,
            content::playlists::{Playlist, PlaylistOwner, get_playlist, search_playlists},
        },
        assert_empty_search_test, setup_test,
    };

    /// Tests creator name prefers display name.
    ///
    /// # Errors
    ///
    /// Returns an error if the assertion fails.
    #[test]
    fn creator_name_prefers_display() -> Result<()> {
        let playlist = Playlist {
            id: None,
            name: None,
            description: None,
            tracks_count: None,
            duration: None,
            is_public: None,
            creator: Some(User {
                id: None,
                credential: None,
                subscription: None,
                display_name: Some("Creator".to_string()),
            }),
            owner: Some(PlaylistOwner {
                id: None,
                name: Some("Owner".to_string()),
            }),
            image: None,
            image_rectangle: None,
            image_rectangle_mini: None,
            images: None,
            images150: None,
            images300: None,
            tracks: None,
            created_at: None,
            updated_at: None,
        };
        ensure!(
            playlist.creator_name() == Some("Creator"),
            "should prefer creator"
        );
        Ok(())
    }

    /// Tests best image URL prefers rectangle banner.
    ///
    /// # Errors
    ///
    /// Returns an error if the assertion fails.
    #[test]
    fn best_image_url_prefers_banner() -> Result<()> {
        let playlist = Playlist {
            id: None,
            name: None,
            description: None,
            tracks_count: None,
            duration: None,
            is_public: None,
            creator: None,
            owner: None,
            image: None,
            image_rectangle: Some(vec!["large.jpg".to_string()]),
            image_rectangle_mini: Some(vec!["mini.jpg".to_string()]),
            images: Some(vec!["small.jpg".to_string()]),
            images150: None,
            images300: None,
            tracks: None,
            created_at: None,
            updated_at: None,
        };
        ensure!(
            playlist.best_image_url(true).as_deref() == Some("large.jpg"),
            "large should prefer banner"
        );
        ensure!(
            playlist.best_image_url(false).as_deref() == Some("mini.jpg"),
            "thumbnail should prefer mini"
        );
        Ok(())
    }

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
