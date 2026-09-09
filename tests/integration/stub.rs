//! Offline stub HTTP client for integration tests.

use std::pin::Pin;

use {reqwest::Response, tracing::info};

use qobuz_api::{
    api::http_client::HttpClient,
    errors::QobuzApiError::{self, UnexpectedApiResponseError},
};

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Stub HTTP client failing all requests with a not-configured error.
#[derive(Clone, Copy, Debug)]
pub struct StubHttpClient;

impl HttpClient for StubHttpClient {
    fn get(
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> BoxFuture<'_, Result<Response, QobuzApiError>> {
        info!(
            url,
            param_count = params.len(),
            "stub HTTP client received GET request"
        );
        stub_not_configured()
    }

    fn post_form(
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> BoxFuture<'_, Result<Response, QobuzApiError>> {
        info!(
            url,
            param_count = params.len(),
            "stub HTTP client received POST request"
        );
        stub_not_configured()
    }

    fn get_with_auth(
        &self,
        url: &str,
        token: &str,
        range: Option<&str>,
    ) -> BoxFuture<'_, Result<Response, QobuzApiError>> {
        info!(
            url,
            token_len = token.len(),
            has_range = range.is_some(),
            "stub HTTP client received authenticated request"
        );
        stub_not_configured()
    }
}

/// Builds a future that always fails with a "not configured" stub error.
fn stub_not_configured() -> BoxFuture<'static, Result<Response, QobuzApiError>> {
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
