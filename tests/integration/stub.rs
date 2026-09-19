//! Offline stub HTTP client for integration tests.

use std::pin::Pin;

use {reqwest::Response, tracing::info};

use qobuz_api::{
    api::http_client::HttpClient,
    errors::QobuzApiError::{self, UnexpectedApiResponseError},
};

/// Stub HTTP client failing all requests with a not-configured error.
#[derive(Clone, Copy, Debug)]
pub struct StubHttpClient;

/// Generates an unauthenticated stub `HttpClient` method.
///
/// Delegates to `stub_with_log` with the given request kind label.
macro_rules! stub_unauth_method {
    ($name:ident, $kind:expr) => {
        fn $name(
            &self,
            url: &str,
            params: &[(&str, &str)],
        ) -> Pin<Box<dyn Future<Output = Result<Response, QobuzApiError>> + Send + '_>> {
            stub_with_log(url, params, $kind)
        }
    };
}

impl HttpClient for StubHttpClient {
    stub_unauth_method!(get, "GET");

    stub_unauth_method!(post_form, "POST");

    fn get_with_auth(
        &self,
        url: &str,
        token: &str,
        range: Option<&str>,
    ) -> Pin<Box<dyn Future<Output = Result<Response, QobuzApiError>> + Send + '_>> {
        info!(
            url,
            token_len = token.len(),
            has_range = range.is_some(),
            "stub HTTP client received authenticated request"
        );
        stub_not_configured()
    }
}

/// Logs a stub GET/POST request and returns a not-configured error future.
///
/// # Arguments
///
/// * `url` - Request URL.
/// * `params` - Key-value query or form parameter pairs.
/// * `kind` - Request kind label (`"GET"` or `"POST"`).
///
/// # Returns
///
/// A future that always fails with a stub not-configured error.
fn stub_with_log(
    url: &str,
    params: &[(&str, &str)],
    kind: &str,
) -> Pin<Box<dyn Future<Output = Result<Response, QobuzApiError>> + Send + 'static>> {
    info!(
        url,
        param_count = params.len(),
        kind,
        "stub HTTP client received request"
    );
    stub_not_configured()
}

/// Builds a future that always fails with a "not configured" stub error.
fn stub_not_configured()
-> Pin<Box<dyn Future<Output = Result<Response, QobuzApiError>> + Send + 'static>> {
    Box::pin(async {
        Err(UnexpectedApiResponseError {
            message: "stub not yet configured".to_string(),
        })
    })
}

#[cfg(test)]
mod tests {
    use qobuz_api::api::http_client::HttpClient;

    use crate::StubHttpClient;

    #[test]
    fn stub_client_implements_transport() {
        fn assert_impl<T: HttpClient>() {}
        assert_impl::<StubHttpClient>();
    }
}
