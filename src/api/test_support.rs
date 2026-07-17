//! Shared test infrastructure: mock HTTP server and service helpers.

use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    thread::{sleep, spawn},
    time::Duration,
};

use anyhow::Result;

use crate::api::{http_client::ReqwestClient, service::QobuzApiService};

#[macro_export]
macro_rules! assert_empty_search {
    ($block:expr) => {{
        let result = $block?;
        let items = result.items.ok_or_else(|| ::anyhow::anyhow!("no items"))?;
        ::anyhow::ensure!(items.is_empty());
    }};
}

#[macro_export]
macro_rules! assert_empty_search_test {
    ($search_fn:path, $query:expr, $body:expr) => {{
        let server = $crate::api::test_support::MockServer::start(200, $body)?;
        let service = $crate::api::test_support::make_service(&server.base_url())?;
        let rt = ::tokio::runtime::Runtime::new()?;
        $crate::assert_empty_search!(rt.block_on($search_fn(&service, $query, None, None)));
    }};
}

#[macro_export]
macro_rules! setup_test {
    ($status:expr, $body:expr, $server:ident, $service:ident, $rt:ident) => {
        let $server = $crate::api::test_support::MockServer::start($status, $body)?;
        let $service = $crate::api::test_support::make_service(&$server.base_url())?;
        let $rt = ::tokio::runtime::Runtime::new()?;
    };
}

pub struct MockServer {
    addr: SocketAddr,
}

impl MockServer {
    pub fn start(status: u16, body: &str) -> Result<Self> {
        Self::start_with_max_requests(status, body, 4)
    }

    pub fn start_with_max_requests(status: u16, body: &str, max_requests: usize) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: \
             {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        spawn_server(listener, response.into_bytes(), max_requests);
        sleep(Duration::from_millis(50));
        Ok(Self { addr })
    }

    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.addr.port())
    }
}

pub struct SequentialMockServer {
    addr: SocketAddr,
}

impl SequentialMockServer {
    pub fn start(responses: Vec<(u16, String)>) -> Result<Self> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;
        let encoded: Vec<Vec<u8>> = responses
            .into_iter()
            .map(|(status, body)| {
                format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: \
                     {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .into_bytes()
            })
            .collect();
        spawn(move || serve_sequential(&listener, &encoded));
        sleep(Duration::from_millis(50));
        Ok(Self { addr })
    }

    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.addr.port())
    }
}

pub fn make_service(base_url: &str) -> Result<QobuzApiService> {
    let client = ReqwestClient::new("test-app-id")?;
    let mut svc = QobuzApiService::new_test(client.into_boxed(), base_url);
    svc.set_auth_token("test-token".to_string());
    Ok(svc)
}

pub fn make_service_without_auth(base_url: &str) -> Result<QobuzApiService> {
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

fn spawn_server(listener: TcpListener, bytes: Vec<u8>, max_requests: usize) {
    spawn(move || serve_loop(&listener, &bytes, max_requests));
}

fn serve_sequential(listener: &TcpListener, responses: &[Vec<u8>]) {
    for response in responses {
        if let Ok(s) = listener.accept().map(|(s, _)| s) {
            serve_response(s, response);
        }
    }
}
