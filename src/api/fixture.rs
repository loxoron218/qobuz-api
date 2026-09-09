//! Shared test infrastructure: mock HTTP server and service helpers.

use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    thread::{sleep, spawn},
    time::Duration,
};

use anyhow::{Result, ensure};

use crate::api::{http_client::ReqwestClient, service::QobuzApiService};

/// Asserts that a search result block contains an empty item list.
///
/// Returns an error if items are missing or non-empty.
#[macro_export]
macro_rules! assert_empty_search {
    ($block:expr) => {{
        let result = $block?;
        let items = result.items.ok_or_else(|| anyhow!("no items"))?;
        ensure!(items.is_empty());
    }};
}

/// Runs a search function against a mock server and asserts empty results.
///
/// Spins up a mock server with the given body and checks the search output.
#[macro_export]
macro_rules! assert_empty_search_test {
    ($search_fn:path, $query:expr, $body:expr) => {{
        let server = $crate::api::fixture::MockServer::start(200, $body)?;
        let service = $crate::api::fixture::make_service(&server.base_url())?;
        let rt = Runtime::new()?;
        $crate::assert_empty_search!(rt.block_on($search_fn(&service, $query, None, None)));
    }};
}

/// Sets up a mock server, service, and Tokio runtime for tests.
///
/// Binds the identifiers to a mock server, authenticated service, and runtime.
#[macro_export]
macro_rules! setup_test {
    ($status:expr, $body:expr, $server:ident, $service:ident, $rt:ident) => {
        let $server = $crate::api::fixture::MockServer::start($status, $body)?;
        let $service = $crate::api::fixture::make_service(&$server.base_url())?;
        let $rt = Runtime::new()?;
    };
}

pub(super) struct MockServer {
    addr: SocketAddr,
}

impl MockServer {
    /// Starts a mock server returning the given status and body.
    ///
    /// # Arguments
    ///
    /// * `status` - HTTP status code to return.
    /// * `body` - Response body to return.
    ///
    /// # Returns
    ///
    /// A running mock server.
    ///
    /// # Errors
    ///
    /// Returns an error if the listener cannot bind or the thread fails to start.
    pub(super) fn start(status: u16, body: &str) -> Result<Self> {
        Self::start_with_max_requests(status, body, 4)
    }

    /// Starts a mock server handling up to `max_requests` requests.
    ///
    /// # Arguments
    ///
    /// * `status` - HTTP status code to return.
    /// * `body` - Response body to return.
    /// * `max_requests` - Maximum requests to serve.
    ///
    /// # Returns
    ///
    /// A running mock server.
    ///
    /// # Errors
    ///
    /// Returns an error if the listener cannot bind or the thread fails to start.
    pub(super) fn start_with_max_requests(
        status: u16,
        body: &str,
        max_requests: usize,
    ) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;
        let response = format_json_response(status, body);
        spawn_server(listener, response, max_requests);
        sleep(Duration::from_millis(50));
        Ok(Self { addr })
    }

    pub(super) fn base_url(&self) -> String {
        socket_base_url(&self.addr)
    }
}

pub(super) struct SequentialMockServer {
    addr: SocketAddr,
}

impl SequentialMockServer {
    /// Starts a sequential mock server returning the given responses in order.
    ///
    /// # Arguments
    ///
    /// * `responses` - Status and body pairs to return in order.
    ///
    /// # Returns
    ///
    /// A running mock server.
    ///
    /// # Errors
    ///
    /// Returns an error if the listener cannot bind or the thread fails to start.
    pub(super) fn start(responses: Vec<(u16, String)>) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;
        let encoded: Vec<Vec<u8>> = responses
            .into_iter()
            .map(|(status, body)| format_json_response(status, &body))
            .collect();
        let handle = spawn(move || serve_sequential(&listener, &encoded));
        ensure!(
            !handle.is_finished(),
            "sequential mock server thread should be running"
        );
        sleep(Duration::from_millis(50));
        Ok(Self { addr })
    }

    pub(super) fn base_url(&self) -> String {
        socket_base_url(&self.addr)
    }
}

/// Formats a minimal HTTP JSON response for mock servers.
///
/// # Arguments
///
/// * `status` - HTTP status code to return.
/// * `body` - Response body to return.
///
/// # Returns
///
/// Raw HTTP response bytes.
fn format_json_response(status: u16, body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: \
         close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

/// Builds a base URL for a bound mock server address.
///
/// # Arguments
///
/// * `addr` - Socket address of the mock server.
///
/// # Returns
///
/// Base URL string pointing at the mock server.
fn socket_base_url(addr: &SocketAddr) -> String {
    format!("http://127.0.0.1:{}", addr.port())
}

/// Creates an authenticated test service pointing at the mock server.
///
/// # Arguments
///
/// * `base_url` - Base URL of the mock server.
///
/// # Returns
///
/// An authenticated test service.
///
/// # Errors
///
/// Returns an error if the HTTP client cannot be created.
pub(super) fn make_service(base_url: &str) -> Result<QobuzApiService> {
    let client = ReqwestClient::new("test-app-id")?;
    let mut svc = QobuzApiService::new_test(client.into_boxed(), base_url);
    svc.set_auth_token("test-token".to_string());
    Ok(svc)
}

/// Creates an unauthenticated test service pointing at the mock server.
///
/// # Arguments
///
/// * `base_url` - Base URL of the mock server.
///
/// # Returns
///
/// A test service without an auth token.
///
/// # Errors
///
/// Returns an error if the HTTP client cannot be created.
pub(super) fn make_service_without_auth(base_url: &str) -> Result<QobuzApiService> {
    let client = ReqwestClient::new("test-app-id")?;
    Ok(QobuzApiService::new_test(client.into_boxed(), base_url))
}

fn serve_response(mut stream: TcpStream, response: &[u8]) {
    let mut buf = [0u8; 8192];
    drop(stream.read(&mut buf));
    drop(stream.write_all(response));
    drop(stream.flush());
}

fn serve_loop(listener: &TcpListener, bytes: &[u8], max_requests: usize) {
    for _ in 0..max_requests {
        if let Ok(s) = listener.accept().map(|(s, _)| s) {
            serve_response(s, bytes);
        }
    }
}

/// Spawns a background thread serving the mock response.
///
/// # Arguments
///
/// * `listener` - TCP listener to accept connections on.
/// * `bytes` - Raw HTTP response bytes to serve.
/// * `max_requests` - Maximum requests to serve.
///
/// # Panics
///
/// Panics if the mock server thread fails to start.
fn spawn_server(listener: TcpListener, bytes: Vec<u8>, max_requests: usize) {
    let handle = spawn(move || serve_loop(&listener, &bytes, max_requests));
    assert!(
        !handle.is_finished(),
        "mock server thread should be running"
    );
}

fn serve_sequential(listener: &TcpListener, responses: &[Vec<u8>]) {
    for response in responses {
        if let Ok(s) = listener.accept().map(|(s, _)| s) {
            serve_response(s, response);
        }
    }
}
