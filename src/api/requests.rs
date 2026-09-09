//! HTTP request primitives: GET, POST, signed GET, response parsing, retry-with-backoff.

#[cfg(test)]
pub mod dispatch;

use std::time::Duration;

use {reqwest::Response, serde::de::DeserializeOwned, tokio::time::sleep};

use crate::{
    api::{
        http_client::HttpClient,
        response::{parse_response, timestamp},
    },
    errors::QobuzApiError::{self, DownloadError, RateLimitError, ResourceNotFoundError},
    signing::sign_request,
};

/// Generates a signed request function with the standard parameter list.
macro_rules! signed_request {
    (
        $(#[$meta:meta])*
        $vis:vis fn $name:ident($client:ident, $base_url:ident, $endpoint:ident, $params:ident, $auth:ident) -> $body:tt
    ) => {
        $(#[$meta])*
        $vis async fn $name<T: DeserializeOwned>(
            $client: &dyn HttpClient,
            $base_url: &str,
            $endpoint: &str,
            $params: &mut Vec<(String, String)>,
            $auth: &RequestAuth<'_>,
        ) -> Result<T, QobuzApiError> $body
    };
}

/// Maximum number of retry attempts on rate limiting.
const MAX_RETRIES: u32 = 3;
/// Base delay in milliseconds for exponential backoff.
const BASE_BACKOFF_MS: u64 = 500;

/// Bundles application credentials and user token for signed API requests.
#[derive(Clone, Copy, Debug)]
pub struct RequestAuth<'a> {
    /// Application ID.
    pub app_id: &'a str,
    /// Application secret for request signing.
    pub app_secret: &'a str,
    /// User authentication token.
    pub user_auth_token: &'a str,
}

/// Appends `app_id`, `request_ts`, and `request_sig` to params for request signing.
///
/// # Arguments
///
/// * `params` - Parameter vector to modify
/// * `method` - HTTP method (e.g., `"GET"`, `"POST"`)
/// * `endpoint` - API endpoint path
/// * `auth` - Application credentials for signing
fn append_signature(
    params: &mut Vec<(String, String)>,
    method: &str,
    endpoint: &str,
    auth: &RequestAuth<'_>,
) {
    params.push(("app_id".to_string(), auth.app_id.to_string()));
    params.push(("request_ts".to_string(), timestamp()));
    let sig = sign_request(method, endpoint, params, auth.app_secret);
    params.push(("request_sig".to_string(), sig));
}

/// Executes a POST request with form parameters and parses the response.
///
/// # Arguments
///
/// * `client` - HTTP client implementation
/// * `base_url` - API base URL
/// * `endpoint` - API endpoint path
/// * `params` - Key-value form parameters
///
/// # Returns
///
/// Parsed JSON response of type `T`.
///
/// # Errors
///
/// Returns a `QobuzApiError` on HTTP failures or JSON parse errors.
async fn execute_post<T: DeserializeOwned>(
    client: &dyn HttpClient,
    base_url: &str,
    endpoint: &str,
    params: &[(String, String)],
) -> Result<T, QobuzApiError> {
    let url = format!("{base_url}{endpoint}");
    let param_refs: Vec<(&str, &str)> = params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    let response = client.post_form(&url, &param_refs).await?;

    parse_response::<T>(response, endpoint).await
}

signed_request!(
    /// Sends a signed GET request and parses the JSON response.
    ///
    /// # Arguments
    ///
    /// * `client` - HTTP client implementation
    /// * `base_url` - API base URL
    /// * `endpoint` - API endpoint path (e.g., `"/album/search"`)
    /// * `params` - Key-value parameter pairs (will be sorted for signing)
    /// * `auth` - Application credentials and user authentication token
    ///
    /// # Returns
    ///
    /// Parsed JSON response of type `T`.
    ///
    /// # Errors
    ///
    /// Returns a `QobuzApiError` on HTTP failures, rate limiting, or JSON parse errors.
    pub fn signed_get(client, base_url, endpoint, params, auth) -> {
        append_signature(params, "GET", endpoint, auth);

        let url = build_url_with_params(base_url, endpoint, params);
        let response = retry_with_backoff(client, &url, auth.user_auth_token).await?;

        parse_response::<T>(response, endpoint).await
    }
);

/// Sends a POST request with form parameters and parses the JSON response.
///
/// # Arguments
///
/// * `client` - HTTP client implementation
/// * `base_url` - API base URL (e.g., `"https://www.qobuz.com/api.json/0.2"`)
/// * `endpoint` - API endpoint path
/// * `params` - Key-value form parameters
/// * `app_id` - Application ID
/// * `user_auth_token` - User authentication token (may be empty for login)
///
/// # Returns
///
/// Parsed JSON response of type `T`.
///
/// # Errors
///
/// Returns a `QobuzApiError` on HTTP failures or JSON parse errors.
pub async fn post<T: DeserializeOwned>(
    client: &dyn HttpClient,
    base_url: &str,
    endpoint: &str,
    params: &mut Vec<(String, String)>,
    app_id: &str,
    user_auth_token: &str,
) -> Result<T, QobuzApiError> {
    params.push(("app_id".to_string(), app_id.to_string()));
    if !user_auth_token.is_empty() {
        params.push(("user_auth_token".to_string(), user_auth_token.to_string()));
    }

    execute_post::<T>(client, base_url, endpoint, params).await
}

signed_request!(
    /// Sends a signed POST request with form parameters and parses the JSON response.
    ///
    /// # Arguments
    ///
    /// * `client` - HTTP client implementation
    /// * `base_url` - API base URL
    /// * `endpoint` - API endpoint path
    /// * `params` - Key-value form parameters (will be sorted for signing)
    /// * `auth` - Application credentials and user authentication token
    ///
    /// # Returns
    ///
    /// Parsed JSON response of type `T`.
    ///
    /// # Errors
    ///
    /// Returns a `QobuzApiError` on HTTP failures, rate limiting, or JSON parse errors.
    pub fn signed_post(client, base_url, endpoint, params, auth) -> {
        params.push((
            "user_auth_token".to_string(),
            auth.user_auth_token.to_string(),
        ));
        append_signature(params, "POST", endpoint, auth);

        execute_post::<T>(client, base_url, endpoint, params).await
    }
);

/// Sends a GET request to an authenticated endpoint for binary download.
///
/// # Arguments
///
/// * `client` - HTTP client implementation
/// * `url` - Full download URL
/// * `token` - User auth token
/// * `range` - Optional Range header value (e.g., `"bytes=1024-"`)
///
/// # Returns
///
/// The raw `reqwest::Response` for streaming.
///
/// # Errors
///
/// Returns a `QobuzApiError` on HTTP failures, 404, or non-success status codes.
pub async fn download_stream(
    client: &dyn HttpClient,
    url: &str,
    token: &str,
    range: Option<&str>,
) -> Result<Response, QobuzApiError> {
    let response = client.get_with_auth(url, token, range).await?;

    let status = response.status();
    if status.is_success() || status.as_u16() == 206 {
        Ok(response)
    } else if status.as_u16() == 404 {
        Err(ResourceNotFoundError {
            resource_type: "file".to_string(),
            resource_id: url.to_string(),
        })
    } else {
        Err(DownloadError {
            message: format!("HTTP {status}"),
        })
    }
}

/// Retries a GET request with exponential backoff on rate limiting (HTTP 429).
///
/// # Arguments
///
/// * `client` - HTTP client implementation
/// * `url` - Full URL with query parameters
/// * `user_auth_token` - User authentication token
///
/// # Returns
///
/// The successful HTTP response.
///
/// # Errors
///
/// Returns a `QobuzApiError::RateLimitError` if all retries are exhausted.
pub async fn retry_with_backoff(
    client: &dyn HttpClient,
    url: &str,
    user_auth_token: &str,
) -> Result<Response, QobuzApiError> {
    let mut last_error: Option<QobuzApiError> = None;

    for attempt in 0..=MAX_RETRIES {
        let response = client.get_with_auth(url, user_auth_token, None).await?;

        let status = response.status();
        if status.as_u16() == 429 {
            let delay = BASE_BACKOFF_MS.saturating_mul(2u64.pow(attempt));
            sleep(Duration::from_millis(delay)).await;
            last_error = Some(RateLimitError {
                message: format!("Rate limited, retry {attempt}/{MAX_RETRIES}"),
            });
            continue;
        }

        return Ok(response);
    }

    Err(last_error.unwrap_or_else(|| RateLimitError {
        message: "Rate limited: retries exhausted".to_string(),
    }))
}

/// Builds a full URL with query parameters from an endpoint path.
///
/// # Arguments
///
/// * `base_url` - API base URL
/// * `endpoint` - API endpoint path (e.g., `"/album/search"`)
/// * `params` - Key-value parameter pairs
///
/// # Returns
///
/// A fully formed URL string with encoded query parameters.
#[must_use]
pub fn build_url_with_params(
    base_url: &str,
    endpoint: &str,
    params: &[(String, String)],
) -> String {
    let mut url = format!("{base_url}{endpoint}");
    if !params.is_empty() {
        let query: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding(k), urlencoding(v)))
            .collect::<Vec<_>>()
            .join("&");
        url.push('?');
        url.push_str(&query);
    }
    url
}

/// URL-encodes special characters in a string.
///
/// # Arguments
///
/// * `s` - The string to encode
///
/// # Returns
///
/// A URL-encoded string with spaces as `+` and special chars as `%XX`.
fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' => "+".to_string(),
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", u32::from(c)),
        })
        .collect()
}
