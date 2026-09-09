//! Favorites tests.

use {
    anyhow::{Result, anyhow, ensure},
    tokio::runtime::Runtime,
};

use crate::api::{
    favorites::{
        add_user_favorites, delete_user_favorites, get_user_favorite_ids, get_user_favorites,
    },
    fixture::{MockServer, make_service, make_service_without_auth},
};

macro_rules! assert_favorites_success {
    ($fn:expr, $ids:expr, $item_type:expr) => {{
        let body = r#"{"status":"success"}"#;
        let server = MockServer::start(200, body)?;
        let service = make_service(&server.base_url())?;
        let rt = Runtime::new()?;
        rt.block_on($fn(&service, $ids, $item_type))?;
        Ok(())
    }};
}

/// Tests add user favorites success.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn add_user_favorites_success() -> Result<()> {
    assert_favorites_success!(add_user_favorites, &[123, 456], "track")
}

/// Tests add user favorites not authenticated.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn add_user_favorites_not_authenticated() -> Result<()> {
    let server = MockServer::start(200, "{}")?;
    let service = make_service_without_auth(&server.base_url())?;
    let rt = Runtime::new()?;
    let result = rt.block_on(add_user_favorites(&service, &[1], "track"));
    ensure!(result.is_err());
    let err = result.err().ok_or_else(|| anyhow!("expected error"))?;
    ensure!(format!("{err}").contains("Not authenticated"));
    Ok(())
}

/// Tests delete user favorites success.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn delete_user_favorites_success() -> Result<()> {
    assert_favorites_success!(delete_user_favorites, &[123], "album")
}

/// Tests delete user favorites not authenticated.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn delete_user_favorites_not_authenticated() -> Result<()> {
    let server = MockServer::start(200, "{}")?;
    let service = make_service_without_auth(&server.base_url())?;
    let rt = Runtime::new()?;
    let result = rt.block_on(delete_user_favorites(&service, &[1], "track"));
    ensure!(result.is_err());
    Ok(())
}

/// Tests get user favorites success.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn get_user_favorites_success() -> Result<()> {
    let body = r#"{"albums":{"items":[{"id":"123","title":"Fav Album"}],"total":1},"artists":null,"tracks":null}"#;
    let server = MockServer::start(200, body)?;
    let service = make_service(&server.base_url())?;
    let rt = Runtime::new()?;
    let result = rt.block_on(get_user_favorites(&service, "album", Some(10), None))?;
    let albums = result.albums.ok_or_else(|| anyhow!("no albums"))?;
    let items = albums.items.ok_or_else(|| anyhow!("no items"))?;
    ensure!(items.len() == 1);
    ensure!(items.first().and_then(|i| i.title.as_deref()) == Some("Fav Album"));
    Ok(())
}

/// Tests get user favorites empty.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn get_user_favorites_empty() -> Result<()> {
    let body = r#"{"albums":{"items":[],"total":0},"artists":null,"tracks":null}"#;
    let server = MockServer::start(200, body)?;
    let service = make_service(&server.base_url())?;
    let rt = Runtime::new()?;
    let result = rt.block_on(get_user_favorites(&service, "album", None, None))?;
    let albums = result.albums.ok_or_else(|| anyhow!("no albums"))?;
    let items = albums.items.ok_or_else(|| anyhow!("no items"))?;
    ensure!(items.is_empty());
    Ok(())
}

/// Tests get user favorites not authenticated.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn get_user_favorites_not_authenticated() -> Result<()> {
    let server = MockServer::start(200, "{}")?;
    let service = make_service_without_auth(&server.base_url())?;
    let rt = Runtime::new()?;
    let result = rt.block_on(get_user_favorites(&service, "track", None, None));
    ensure!(result.is_err());
    Ok(())
}

/// Tests get user favorite ids success.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn get_user_favorite_ids_success() -> Result<()> {
    let body = r#"{"album_ids":[1,2,3],"artist_ids":[4,5],"track_ids":[6,7,8,9],"albums":null,"artists":null,"tracks":null}"#;
    let server = MockServer::start(200, body)?;
    let service = make_service(&server.base_url())?;
    let rt = Runtime::new()?;
    let result = rt.block_on(get_user_favorite_ids(&service))?;
    let album_ids = result.album_ids.ok_or_else(|| anyhow!("no album_ids"))?;
    ensure!(album_ids == vec![1, 2, 3]);
    let track_ids = result.track_ids.ok_or_else(|| anyhow!("no track_ids"))?;
    ensure!(track_ids == vec![6, 7, 8, 9]);
    Ok(())
}

/// Tests get user favorite ids not authenticated.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn get_user_favorite_ids_not_authenticated() -> Result<()> {
    let server = MockServer::start(200, "{}")?;
    let service = make_service_without_auth(&server.base_url())?;
    let rt = Runtime::new()?;
    let result = rt.block_on(get_user_favorite_ids(&service));
    ensure!(result.is_err());
    Ok(())
}

/// Tests add favorites api error.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn add_favorites_api_error() -> Result<()> {
    let body = r#"{"status":"error","code":400,"message":"Invalid item type"}"#;
    let server = MockServer::start(400, body)?;
    let service = make_service(&server.base_url())?;
    let rt = Runtime::new()?;
    let result = rt.block_on(add_user_favorites(&service, &[1], "invalid_type"));
    ensure!(result.is_err());
    let err = result.err().ok_or_else(|| anyhow!("expected error"))?;
    ensure!(format!("{err}").contains("Invalid item type"));
    Ok(())
}

/// Tests get favorites with pagination.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn get_favorites_with_pagination() -> Result<()> {
    let body = r#"{"tracks":{"items":[{"id":1,"title":"Song"}],"total":100,"limit":1,"offset":0},"albums":null,"artists":null}"#;
    let server = MockServer::start(200, body)?;
    let service = make_service(&server.base_url())?;
    let rt = Runtime::new()?;
    let result = rt.block_on(get_user_favorites(&service, "track", Some(1), Some(0)))?;
    let tracks = result.tracks.ok_or_else(|| anyhow!("no tracks"))?;
    ensure!(tracks.total == Some(100));
    ensure!(tracks.limit == Some(1));
    ensure!(tracks.offset == Some(0));
    Ok(())
}
