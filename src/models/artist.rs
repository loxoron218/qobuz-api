//! Artist data model.

use serde::{Deserialize, Serialize};

use crate::models::{
    album::{Album, Image},
    deserialization::{deserialize_flexible_name, deserialize_picture},
    search::ItemSearchResult,
};

/// A music artist.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Artist {
    /// Unique artist identifier.
    pub id: Option<i32>,
    /// Artist name (may be a plain string or `{"display":"Name"}` from the API).
    #[serde(default, deserialize_with = "deserialize_flexible_name")]
    pub name: Option<String>,
    /// URL-friendly name.
    pub slug: Option<String>,
    /// Artist picture (may be a string URL or an Image object from the API).
    #[serde(default, deserialize_with = "deserialize_picture")]
    pub picture: Option<Image>,
    /// Artist image (alternate field).
    pub image: Option<Image>,
    /// Biography text.
    pub biography: Option<Biography>,
    /// Number of albums.
    pub albums_count: Option<i32>,
    /// Artist roles (main-artist, composer, etc.).
    pub roles: Option<Vec<String>>,
    /// Associated albums.
    pub albums: Option<ItemSearchResult<Box<Album>>>,
}

/// Artist biography text.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Biography {
    /// Biography text content.
    pub text: Option<String>,
    /// Summary text.
    pub summary: Option<String>,
}
