//! Playlist data model.

use serde::{Deserialize, Serialize};

use crate::models::{
    album::Image, deserialization::deserialize_flexible_string_id, search::ItemSearchResult,
    subscription::User, track::Track,
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

#[cfg(test)]
mod tests {
    use anyhow::{Result, ensure};

    use crate::models::{
        playlist::{Playlist, PlaylistOwner},
        subscription::User,
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
}
