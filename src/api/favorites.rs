//! Favorites management: add, remove, and retrieve favorites.

#[cfg(test)]
pub mod collection;

use {
    serde_json::Value,
    tracing::{info, instrument, warn},
};

use crate::{
    api::{
        content::{catalog::UserFavorites, push_pagination_params},
        requests::{RequestAuth, signed_get, signed_post},
        service::QobuzApiService,
    },
    errors::QobuzApiError,
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

    let response: Value = signed_post::<Value>(
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
    if !response.is_object() {
        warn!("favorites modify endpoint returned non-object response");
    }

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

    signed_get(
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
