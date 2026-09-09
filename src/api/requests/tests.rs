//! Request retry and download tests.

use {
    anyhow::{Result, anyhow, ensure},
    reqwest::Response,
    tokio::runtime::Runtime,
};

use crate::{
    api::{
        requests::{download_stream, retry_with_backoff},
        service::QobuzApiService,
        test_support::{MockServer, SequentialMockServer, make_service},
    },
    errors::QobuzApiError,
};

fn rate_limit_response() -> (u16, String) {
    (
        429,
        r#"{"status":"error","message":"rate limited"}"#.to_string(),
    )
}

/// Makes a test request against the mock server.
///
/// # Arguments
///
/// * `service` - Test service pointing at the mock server.
///
/// # Errors
///
/// Returns a `QobuzApiError` if the request fails.
fn make_test_request(service: &QobuzApiService) -> Result<Response, QobuzApiError> {
    let rt = Runtime::new()?;
    let client = service.http_client();
    rt.block_on(retry_with_backoff(
        client,
        &format!("{}/test", service.base_url()),
        "token",
    ))
}

/// Tests rate limit retry exhausts retries.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn rate_limit_retry_exhausts_retries() -> Result<()> {
    let server = SequentialMockServer::start(vec![
        rate_limit_response(),
        rate_limit_response(),
        rate_limit_response(),
        rate_limit_response(),
    ])?;
    let service = make_service(&server.base_url())?;
    let result = make_test_request(&service);
    let err = result.err().ok_or_else(|| anyhow!("expected error"))?;
    ensure!(format!("{err}").contains("Rate limited"));
    Ok(())
}

/// Tests rate limit retry succeeds after backoff.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn rate_limit_retry_succeeds_after_backoff() -> Result<()> {
    let server = SequentialMockServer::start(vec![
        rate_limit_response(),
        rate_limit_response(),
        (
            200,
            r#"{"url":"https://example.com/file.flac"}"#.to_string(),
        ),
    ])?;
    let service = make_service(&server.base_url())?;
    let result = make_test_request(&service)?;
    ensure!(result.status().is_success());
    Ok(())
}

/// Tests download stream returns success on 200.
///
/// # Errors
///
/// Returns an error if the mock setup fails.
#[test]
fn download_stream_succeeds_on_ok() -> Result<()> {
    let server = MockServer::start(200, "bytes")?;
    let service = make_service(&server.base_url())?;
    let rt = Runtime::new()?;
    let response = rt.block_on(download_stream(
        service.http_client(),
        &format!("{}/file", service.base_url()),
        "token",
        None,
    ))?;
    ensure!(response.status().is_success(), "expected success status");
    Ok(())
}

/// Tests download stream errors on 404.
///
/// # Errors
///
/// Returns an error if the mock setup fails.
#[test]
fn download_stream_errors_on_not_found() -> Result<()> {
    let server = MockServer::start(404, "missing")?;
    let service = make_service(&server.base_url())?;
    let rt = Runtime::new()?;
    let result = rt.block_on(download_stream(
        service.http_client(),
        &format!("{}/file", service.base_url()),
        "token",
        None,
    ));
    ensure!(result.is_err(), "expected not-found error");
    Ok(())
}
