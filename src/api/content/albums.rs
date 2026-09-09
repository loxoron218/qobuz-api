//! Album search and browse operations.

use {
    serde::{Deserialize, Serialize},
    serde_json::Value,
};

use crate::{
    api::{
        content::{
            artists::Artist,
            catalog::{AlbumSearchResponse, ItemSearchResult},
            get_by_id, search,
        },
        service::QobuzApiService,
    },
    errors::QobuzApiError,
};

/// A music album with full metadata.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Album {
    /// Unique album identifier.
    pub id: Option<String>,
    /// Album title.
    pub title: Option<String>,
    /// Version/subtitle.
    pub version: Option<String>,
    /// Universal Product Code.
    pub upc: Option<String>,
    /// Qobuz web URL.
    pub url: Option<String>,
    /// Primary artist.
    pub artist: Option<Box<Artist>>,
    /// All artists.
    pub artists: Option<Vec<Box<Artist>>>,
    /// Primary composer.
    pub composer: Option<Box<Artist>>,
    /// Record label.
    pub label: Option<Label>,
    /// Primary genre.
    pub genre: Option<Genre>,
    /// All genres.
    pub genres: Option<Vec<Genre>>,
    /// Cover art URLs.
    pub image: Option<Image>,
    /// Total duration in seconds.
    pub duration: Option<i32>,
    /// Number of tracks.
    pub tracks_count: Option<i32>,
    /// Number of discs.
    pub media_count: Option<i32>,
    /// Original release date.
    pub release_date_original: Option<String>,
    /// Streaming availability date.
    pub release_date_stream: Option<String>,
    /// Download availability date.
    pub release_date_download: Option<String>,
    /// Product type identifier.
    pub product_type: Option<String>,
    /// Release type (album, single, etc.).
    pub release_type: Option<String>,
    /// Hi-Res audio available.
    pub hires: Option<bool>,
    /// Hi-Res streaming available.
    pub hires_streamable: Option<bool>,
    /// Download available.
    pub downloadable: Option<bool>,
    /// Streaming available.
    pub streamable: Option<bool>,
    /// List of track IDs (when requested with extra).
    pub track_ids: Option<Vec<i32>>,
    /// Product URL for commercial information.
    pub product_url: Option<String>,
    /// Unix timestamp of when the album was released.
    pub released_at: Option<i64>,
    /// Copyright notice.
    pub copyright: Option<String>,
    /// Sales metadata.
    pub product_sales_factors: Option<Value>,
    /// Maximum available bit depth.
    pub maximum_bit_depth: Option<i32>,
    /// Maximum available sample rate.
    pub maximum_sampling_rate: Option<f64>,
    /// Maximum channel count.
    pub maximum_channel_count: Option<i32>,
}

impl Album {
    /// Returns the file extension for the given quality format ID.
    ///
    /// # Arguments
    ///
    /// * `format_id` - Quality format ID (5=MP3, everything else=FLAC)
    ///
    /// # Returns
    ///
    /// The file extension as a static string slice (`"mp3"` or `"flac"`).
    #[must_use]
    pub const fn extension_for_format(format_id: i32) -> &'static str {
        match format_id {
            5 => "mp3",
            _ => "flac",
        }
    }
}

/// Music genre.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Genre {
    /// Genre ID.
    pub id: Option<i32>,
    /// Genre name.
    pub name: Option<String>,
    /// URL-friendly name.
    pub slug: Option<String>,
    /// Display color.
    pub color: Option<String>,
}

/// Cover art URLs in multiple sizes.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Image {
    /// Small thumbnail URL.
    pub small: Option<String>,
    /// Thumbnail URL.
    pub thumbnail: Option<String>,
    /// Medium size URL.
    pub medium: Option<String>,
    /// Large size URL.
    pub large: Option<String>,
    /// Extra-large URL.
    #[serde(rename = "extralarge")]
    pub extra_large: Option<String>,
    /// Highest resolution URL.
    pub mega: Option<String>,
    /// Back cover URL.
    pub back: Option<String>,
    /// Generic/poster URL (returned by some endpoints instead of the size-specific map).
    pub url: Option<String>,
}

/// Record label.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Label {
    /// Label ID.
    pub id: Option<i32>,
    /// Label name.
    pub name: Option<String>,
    /// URL-friendly name.
    pub slug: Option<String>,
}

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
            content::albums::{Album, get_album, search_albums},
            fixture::{MockServer, make_service},
        },
        assert_empty_search_test,
    };

    /// Tests file extension mapping for quality IDs.
    ///
    /// # Errors
    ///
    /// Returns an error if the assertion fails.
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

    /// Tests search albums deserializes results.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
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

    /// Tests search albums empty results.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
    #[test]
    fn search_albums_empty_results() -> Result<()> {
        assert_empty_search_test!(
            search_albums,
            "Nonexistent",
            r#"{"albums":{"items":[],"total":0}}"#
        );
        Ok(())
    }

    /// Tests search albums error response.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
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

    /// Tests get album by id.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
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

    /// Tests get album with extra param.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
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

    /// Tests get album not found.
    ///
    /// # Errors
    ///
    /// Returns an error if the test setup or assertion fails.
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
