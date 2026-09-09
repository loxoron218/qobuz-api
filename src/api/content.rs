//! Content API operations: search and browse for albums, artists, tracks, playlists.

pub mod album_download;
pub mod albums;
pub mod artist_download;
pub mod artists;
pub mod catalog;
pub mod cover;
pub mod download_io;
pub mod playlist_download;
pub mod playlists;
pub mod stream;
pub mod tracks;

use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

use serde::de::DeserializeOwned;

use crate::{
    api::{
        requests::{RequestAuth, signed_get},
        service::QobuzApiService,
    },
    errors::QobuzApiError::{self, Canceled},
};

/// Checks the cancellation flag and returns `Canceled` if the download was cancelled.
///
/// # Arguments
///
/// * `cancel` - Optional cancellation flag
///
/// # Returns
///
/// `Ok(())` if not cancelled.
///
/// # Errors
///
/// Returns `QobuzApiError::Canceled` if the cancellation flag is set.
pub fn check_cancel(cancel: Option<&AtomicBool>) -> Result<(), QobuzApiError> {
    if cancel.is_some_and(|c| c.load(Relaxed)) {
        return Err(Canceled);
    }
    Ok(())
}

/// Checks cancellation before and after an async fetch, discarding the fetch result on cancel.
///
/// # Arguments
///
/// * `cancel` - Optional cancellation flag
/// * `f` - Async closure producing the value to fetch
///
/// # Returns
///
/// The fetch result if not cancelled.
///
/// # Errors
///
/// Returns `QobuzApiError::Canceled` if cancelled before or after the fetch.
pub async fn fetch_with_cancel<T, F, Fut>(
    cancel: Option<&AtomicBool>,
    f: F,
) -> Result<T, QobuzApiError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, QobuzApiError>>,
{
    check_cancel(cancel)?;
    let result = f().await?;
    check_cancel(cancel)?;
    Ok(result)
}

/// Appends optional `limit` and `offset` pagination parameters to a params vector.
///
/// # Arguments
///
/// * `params` - Parameter vector to modify
/// * `limit` - Maximum number of results (if provided)
/// * `offset` - Pagination offset (if provided)
pub fn push_pagination_params(
    params: &mut Vec<(String, String)>,
    limit: Option<i32>,
    offset: Option<i32>,
) {
    if let Some(l) = limit {
        params.push(("limit".to_string(), l.to_string()));
    }
    if let Some(o) = offset {
        params.push(("offset".to_string(), o.to_string()));
    }
}

/// Retrieves the auth token from the service and sends a signed GET request.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `endpoint` - API endpoint path
/// * `params` - Key-value parameter pairs (will be sorted for signing)
///
/// # Returns
///
/// The deserialized response of type `T`.
///
/// # Errors
///
/// Returns a `QobuzApiError` if not authenticated or the API request fails.
async fn do_signed_get<T: DeserializeOwned>(
    service: &QobuzApiService,
    endpoint: &str,
    params: &mut Vec<(String, String)>,
) -> Result<T, QobuzApiError> {
    let token = service.require_auth_token()?;

    signed_get(
        service.http_client(),
        service.base_url(),
        endpoint,
        params,
        &RequestAuth {
            app_id: &service.app_id,
            app_secret: service.app_secret(),
            user_auth_token: token,
        },
    )
    .await
}

/// Sends a search request with query, limit, and offset parameters.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `endpoint` - API endpoint path (e.g., `"/album/search"`)
/// * `query` - Search query string
/// * `limit` - Maximum number of results to return
/// * `offset` - Pagination offset
///
/// # Returns
///
/// The deserialized response of type `T`.
///
/// # Errors
///
/// Returns a `QobuzApiError` if not authenticated or the API request fails.
pub async fn search<T: DeserializeOwned>(
    service: &QobuzApiService,
    endpoint: &str,
    query: &str,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<T, QobuzApiError> {
    let mut params = vec![("query".to_string(), query.to_string())];
    push_pagination_params(&mut params, limit, offset);

    do_signed_get(service, endpoint, &mut params).await
}

/// Sends a get-by-ID request with an optional extra parameter.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `endpoint` - API endpoint path (e.g., `"/album/get"`)
/// * `id_field` - Parameter name for the ID (e.g., `"album_id"`)
/// * `id` - Resource identifier
/// * `extra` - Optional extra fields to include
///
/// # Returns
///
/// The deserialized response of type `T`.
///
/// # Errors
///
/// Returns a `QobuzApiError` if not authenticated or the API request fails.
pub async fn get_by_id<T: DeserializeOwned, I: ToString>(
    service: &QobuzApiService,
    endpoint: &str,
    id_field: &str,
    id: I,
    extra: Option<&str>,
) -> Result<T, QobuzApiError> {
    let mut params = vec![(id_field.to_string(), id.to_string())];
    if let Some(e) = extra {
        params.push(("extra".to_string(), e.to_string()));
    }

    do_signed_get(service, endpoint, &mut params).await
}

/// Sends a paginated request by ID with limit and offset parameters.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `endpoint` - API endpoint path (e.g., `"/artist/getReleasesList"`)
/// * `id_field` - Parameter name for the ID (e.g., `"artist_id"`)
/// * `id` - Resource identifier
/// * `limit` - Maximum number of results to return
/// * `offset` - Pagination offset
///
/// # Returns
///
/// The deserialized response of type `T`.
///
/// # Errors
///
/// Returns a `QobuzApiError` if not authenticated or the API request fails.
pub async fn paginated<T: DeserializeOwned, I: ToString>(
    service: &QobuzApiService,
    endpoint: &str,
    id_field: &str,
    id: I,
    limit: Option<i32>,
    offset: Option<i32>,
) -> Result<T, QobuzApiError> {
    let mut params = vec![(id_field.to_string(), id.to_string())];
    push_pagination_params(&mut params, limit, offset);

    do_signed_get(service, endpoint, &mut params).await
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicBool;

    use {
        anyhow::{Result, anyhow, ensure},
        tokio::runtime::Runtime,
    };

    use crate::{
        api::content::{check_cancel, fetch_with_cancel},
        errors::QobuzApiError::{self, Canceled as CanceledVariant},
    };

    /// Tests cancellation check passes without a flag.
    ///
    /// # Errors
    ///
    /// Returns an error if the assertion fails.
    #[test]
    fn check_cancel_without_flag_succeeds() -> Result<()> {
        check_cancel(None)?;
        Ok(())
    }

    /// Tests cancellation check fails when the flag is set.
    ///
    /// # Errors
    ///
    /// Returns an error if the assertion fails.
    #[test]
    fn check_cancel_with_flag_fails() -> Result<()> {
        let cancel = AtomicBool::new(true);
        let result = check_cancel(Some(&cancel));
        ensure!(result.is_err(), "expected canceled error");
        let err = result.err().ok_or_else(|| anyhow!("expected error"))?;
        ensure!(matches!(err, CanceledVariant), "expected Canceled variant");
        Ok(())
    }

    /// Tests fetch wrapper returns the value when not cancelled.
    ///
    /// # Errors
    ///
    /// Returns an error if the assertion fails.
    #[test]
    fn fetch_with_cancel_returns_value() -> Result<()> {
        let rt = Runtime::new()?;
        let value = rt.block_on(fetch_with_cancel(None, async || {
            Ok::<i32, QobuzApiError>(7)
        }))?;
        ensure!(value == 7, "unexpected fetch value");
        Ok(())
    }

    /// Tests fetch wrapper fails when cancelled before fetch.
    ///
    /// # Errors
    ///
    /// Returns an error if the assertion fails.
    #[test]
    fn fetch_with_cancel_respects_cancel() -> Result<()> {
        let cancel = AtomicBool::new(true);
        let rt = Runtime::new()?;
        let result = rt.block_on(fetch_with_cancel(Some(&cancel), async || {
            Ok::<i32, QobuzApiError>(7)
        }));
        ensure!(result.is_err(), "expected canceled error");
        Ok(())
    }
}
