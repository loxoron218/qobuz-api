//! Favorites management: add, remove, and retrieve favorites.

use std::string::ToString;

use {
    serde_json::Value,
    tracing::{info, instrument},
};

use crate::{
    api::{
        content::push_pagination_params,
        requests::{self, RequestAuth},
        service::QobuzApiService,
    },
    errors::QobuzApiError,
    models::search::UserFavorites,
};

/// Sends a signed POST to add or remove favorites and logs the result.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `item_ids` - IDs of items to modify
/// * `item_type` - Type of items (`"album"`, `"artist"`, or `"track"`)
/// * `endpoint` - API endpoint path (e.g., `"/favorite/create"`)
/// * `log_action` - Description to log on success
///
/// # Returns
///
/// `Ok(())` on success.
///
/// # Errors
///
/// Returns a `QobuzApiError` if not authenticated or the API request fails.
async fn modify_favorites(
    service: &QobuzApiService,
    item_ids: &[i32],
    item_type: &str,
    endpoint: &str,
    log_action: &str,
) -> Result<(), QobuzApiError> {
    let token = service.require_auth_token()?;

    let ids: String = item_ids
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");

    let mut params = vec![
        ("item_ids".to_string(), ids),
        ("item_type".to_string(), item_type.to_string()),
    ];

    requests::signed_post::<Value>(
        service.http_client(),
        service.base_url(),
        endpoint,
        &mut params,
        &RequestAuth {
            app_id: &service.app_id,
            app_secret: service.app_secret(),
            user_auth_token: token,
        },
    )
    .await?;

    info!(item_type, count = item_ids.len(), "{log_action}");
    Ok(())
}

/// Adds items to the user's favorites.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `item_ids` - IDs of items to favorite
/// * `item_type` - Type of items (`"album"`, `"artist"`, or `"track"`)
///
/// # Returns
///
/// `Ok(())` on success.
///
/// # Errors
///
/// Returns a `QobuzApiError` if not authenticated or the API request fails.
#[instrument(skip(service), fields(item_type, count = item_ids.len()))]
pub async fn add_user_favorites(
    service: &QobuzApiService,
    item_ids: &[i32],
    item_type: &str,
) -> Result<(), QobuzApiError> {
    modify_favorites(
        service,
        item_ids,
        item_type,
        "/favorite/create",
        "Added favorites",
    )
    .await
}

/// Removes items from the user's favorites.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `item_ids` - IDs of items to remove
/// * `item_type` - Type of items (`"album"`, `"artist"`, or `"track"`)
///
/// # Returns
///
/// `Ok(())` on success.
///
/// # Errors
///
/// Returns a `QobuzApiError` if not authenticated or the API request fails.
#[instrument(skip(service), fields(item_type, count = item_ids.len()))]
pub async fn delete_user_favorites(
    service: &QobuzApiService,
    item_ids: &[i32],
    item_type: &str,
) -> Result<(), QobuzApiError> {
    modify_favorites(
        service,
        item_ids,
        item_type,
        "/favorite/delete",
        "Deleted favorites",
    )
    .await
}

/// Fetches user favorites with the given parameters.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `params` - Query parameters for the favorites request
///
/// # Returns
///
/// The user's favorited items.
///
/// # Errors
///
/// Returns a `QobuzApiError` if not authenticated or the API request fails.
async fn fetch_user_favorites(
    service: &QobuzApiService,
    params: &mut Vec<(String, String)>,
) -> Result<UserFavorites, QobuzApiError> {
    let token = service.require_auth_token()?;

    requests::signed_get(
        service.http_client(),
        service.base_url(),
        "/favorite/getUserFavorites",
        params,
        &RequestAuth {
            app_id: &service.app_id,
            app_secret: service.app_secret(),
            user_auth_token: token,
        },
    )
    .await
}

/// Retrieves the user's favorites list.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `item_type` - Type of items to retrieve
/// * `limit` - Maximum number of results
/// * `offset` - Pagination offset
///
/// # Returns
///
/// The user's favorited items grouped by type.
///
/// # Errors
///
/// Returns a `QobuzApiError` if not authenticated or the API request fails.
#[instrument(skip(service), fields(item_type, limit, offset))]
pub async fn get_user_favorites(
    service: &QobuzApiService,
    item_type: &str,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<UserFavorites, QobuzApiError> {
    let mut params: Vec<(String, String)> = vec![("type".to_string(), item_type.to_string())];
    push_pagination_params(&mut params, limit, offset);
    fetch_user_favorites(service, &mut params).await
}

/// Retrieves only the favorite IDs grouped by type.
///
/// # Arguments
///
/// * `service` - Authenticated API service
///
/// # Returns
///
/// The user's favorite IDs grouped by type.
///
/// # Errors
///
/// Returns a `QobuzApiError` if not authenticated or the API request fails.
#[instrument(skip(service))]
pub async fn get_user_favorite_ids(
    service: &QobuzApiService,
) -> Result<UserFavorites, QobuzApiError> {
    let mut params: Vec<(String, String)> = vec![("type".to_string(), "ids".to_string())];
    fetch_user_favorites(service, &mut params).await
}

#[cfg(test)]
mod tests {
    use {
        anyhow::{Result, anyhow, ensure},
        tokio::runtime::Runtime,
    };

    use crate::api::{
        favorites::{
            add_user_favorites, delete_user_favorites, get_user_favorite_ids, get_user_favorites,
        },
        test_support::{MockServer, make_service, make_service_without_auth},
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

    #[test]
    fn add_user_favorites_success() -> Result<()> {
        assert_favorites_success!(add_user_favorites, &[123, 456], "track")
    }

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

    #[test]
    fn delete_user_favorites_success() -> Result<()> {
        assert_favorites_success!(delete_user_favorites, &[123], "album")
    }

    #[test]
    fn delete_user_favorites_not_authenticated() -> Result<()> {
        let server = MockServer::start(200, "{}")?;
        let service = make_service_without_auth(&server.base_url())?;
        let rt = Runtime::new()?;
        let result = rt.block_on(delete_user_favorites(&service, &[1], "track"));
        ensure!(result.is_err());
        Ok(())
    }

    #[test]
    fn get_user_favorites_success() -> Result<()> {
        let body = r#"{"albums":{"items":[{"id":"123","title":"Fav Album"}],"total":1},"artists":null,"tracks":null}"#;
        let server = MockServer::start(200, body)?;
        let service = make_service(&server.base_url())?;
        let rt = Runtime::new()?;
        let result = rt.block_on(get_user_favorites(&service, "album", Some(10), None))?;
        let albums = result.albums.ok_or_else(|| anyhow::anyhow!("no albums"))?;
        let items = albums.items.ok_or_else(|| anyhow::anyhow!("no items"))?;
        ensure!(items.len() == 1);
        ensure!(items.first().and_then(|i| i.title.as_deref()) == Some("Fav Album"));
        Ok(())
    }

    #[test]
    fn get_user_favorites_empty() -> Result<()> {
        let body = r#"{"albums":{"items":[],"total":0},"artists":null,"tracks":null}"#;
        let server = MockServer::start(200, body)?;
        let service = make_service(&server.base_url())?;
        let rt = Runtime::new()?;
        let result = rt.block_on(get_user_favorites(&service, "album", None, None))?;
        let albums = result.albums.ok_or_else(|| anyhow::anyhow!("no albums"))?;
        let items = albums.items.ok_or_else(|| anyhow::anyhow!("no items"))?;
        ensure!(items.is_empty());
        Ok(())
    }

    #[test]
    fn get_user_favorites_not_authenticated() -> Result<()> {
        let server = MockServer::start(200, "{}")?;
        let service = make_service_without_auth(&server.base_url())?;
        let rt = Runtime::new()?;
        let result = rt.block_on(get_user_favorites(&service, "track", None, None));
        ensure!(result.is_err());
        Ok(())
    }

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

    #[test]
    fn get_user_favorite_ids_not_authenticated() -> Result<()> {
        let server = MockServer::start(200, "{}")?;
        let service = make_service_without_auth(&server.base_url())?;
        let rt = Runtime::new()?;
        let result = rt.block_on(get_user_favorite_ids(&service));
        ensure!(result.is_err());
        Ok(())
    }

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

    #[test]
    fn get_favorites_with_pagination() -> Result<()> {
        let body = r#"{"tracks":{"items":[{"id":1,"title":"Song"}],"total":100,"limit":1,"offset":0},"albums":null,"artists":null}"#;
        let server = MockServer::start(200, body)?;
        let service = make_service(&server.base_url())?;
        let rt = Runtime::new()?;
        let result = rt.block_on(get_user_favorites(&service, "track", Some(1), Some(0)))?;
        let tracks = result.tracks.ok_or_else(|| anyhow::anyhow!("no tracks"))?;
        ensure!(tracks.total == Some(100));
        ensure!(tracks.limit == Some(1));
        ensure!(tracks.offset == Some(0));
        Ok(())
    }
}
