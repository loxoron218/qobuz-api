//! HTTP client trait abstraction for deterministic testing.

use std::pin::Pin;

use {
    reqwest::{
        Client, Response,
        header::{HeaderMap, HeaderName, HeaderValue},
    },
    tracing::warn,
};

use crate::errors::QobuzApiError;

/// Type alias for a pinned, boxed, `Send` future.
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// HTTP client trait enabling deterministic testing via mock implementations.
///
/// Uses boxed futures instead of `async fn` to remain object-safe for `dyn` dispatch.
pub trait HttpClient: Send + Sync {
    /// Sends a GET request with query parameters.
    ///
    /// # Arguments
    ///
    /// * `url` - Full URL to send the request to
    /// * `params` - Key-value query parameter pairs
    ///
    /// # Returns
    ///
    /// The HTTP response.
    fn get(
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> BoxFuture<'_, Result<Response, QobuzApiError>>;

    /// Sends a POST request with form parameters.
    ///
    /// # Arguments
    ///
    /// * `url` - Full URL to send the request to
    /// * `params` - Key-value form parameter pairs
    ///
    /// # Returns
    ///
    /// The HTTP response.
    fn post_form(
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> BoxFuture<'_, Result<Response, QobuzApiError>>;

    /// Sends an authenticated GET request with optional Range header.
    ///
    /// # Arguments
    ///
    /// * `url` - Full URL to send the request to
    /// * `token` - Bearer authentication token
    /// * `range` - Optional Range header value (e.g., `"bytes=1024-"`)
    ///
    /// # Returns
    ///
    /// The HTTP response.
    fn get_with_auth(
        &self,
        url: &str,
        token: &str,
        range: Option<&str>,
    ) -> BoxFuture<'_, Result<Response, QobuzApiError>>;
}

/// Production HTTP client wrapping `reqwest::Client`.
#[derive(Debug)]
pub struct ReqwestClient {
    /// Inner reqwest client with connection pooling.
    inner: Client,
}

impl ReqwestClient {
    /// Creates a new `ReqwestClient` with default headers and connection pooling.
    ///
    /// # Arguments
    ///
    /// * `app_id` - Qobuz application ID sent as `x-app-id` header on every request
    ///
    /// # Returns
    ///
    /// A `ReqwestClient` ready for making HTTP requests.
    ///
    /// # Errors
    ///
    /// Returns a `QobuzApiError` if the reqwest client builder fails.
    pub fn new(app_id: &str) -> Result<Self, QobuzApiError> {
        let mut default_headers = HeaderMap::new();
        if let Ok(val) = HeaderValue::from_str(app_id)
            && default_headers
                .insert(HeaderName::from_static("x-app-id"), val)
                .is_some()
        {
            warn!("fresh header map already contained x-app-id");
        }

        let inner = Client::builder()
            .user_agent(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/110.0",
            )
            .default_headers(default_headers)
            .build()?;

        Ok(Self { inner })
    }

    /// Boxes this client as a `dyn HttpClient`.
    ///
    /// # Returns
    ///
    /// A `Box<dyn HttpClient>` suitable for trait-object dispatch.
    #[must_use]
    pub fn into_boxed(self) -> Box<dyn HttpClient> {
        Box::new(self)
    }
}

impl HttpClient for ReqwestClient {
    fn get(
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> BoxFuture<'_, Result<Response, QobuzApiError>> {
        let fut = self.inner.get(url).query(params).send();
        Box::pin(async move {
            let resp = fut.await?;
            Ok(resp)
        })
    }

    fn post_form(
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> BoxFuture<'_, Result<Response, QobuzApiError>> {
        let params: Vec<(String, String)> = params
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();

        let fut = self.inner.post(url).form(&params);
        Box::pin(async move {
            let resp = fut.send().await?;
            Ok(resp)
        })
    }

    fn get_with_auth(
        &self,
        url: &str,
        token: &str,
        range: Option<&str>,
    ) -> BoxFuture<'_, Result<Response, QobuzApiError>> {
        let mut req = self.inner.get(url).header("X-User-Auth-Token", token);

        if let Some(r) = range {
            req = req.header("Range", r);
        }

        let fut = req.send();
        Box::pin(async move {
            let resp = fut.await?;
            Ok(resp)
        })
    }
}
