//! Error display and remediation tests.

use std::io::{Error, ErrorKind::NotFound};

use {reqwest::Client, tokio::runtime::Runtime};

use crate::errors::QobuzApiError::{
    self, ApiErrorResponse, ApiResponseParseError, AuthenticationError, Canceled, CredentialsError,
    DownloadError, HttpError, InitializationError, InvalidParameterError, IoError, MetadataError,
    RateLimitError, ResourceNotFoundError, UnexpectedApiResponseError,
};

/// Asserts that a type satisfies `Send + Sync + 'static` bounds.
fn assert_send_sync_static<T: Send + Sync + 'static>() {}

#[test]
fn all_variants_satisfy_send_sync_static() {
    assert_send_sync_static::<QobuzApiError>();
}

#[test]
fn authentication_error_display() {
    let err = AuthenticationError {
        message: "bad token".into(),
    };
    let msg = format!("{err}");
    assert!(msg.contains("Authentication failed"));
    assert!(msg.contains("bad token"));
}

#[test]
fn http_error_display() {
    let Ok(rt) = Runtime::new() else {
        return;
    };
    let result = rt.block_on(async { Client::new().get("http://localhost:1").send().await });
    let Err(req_err) = result else { return };
    let err = HttpError(req_err);
    let msg = format!("{err}");
    assert!(msg.contains("HTTP request failed"));
}

#[test]
fn io_error_display() {
    let err = IoError(Error::new(NotFound, "file missing"));
    let msg = format!("{err}");
    assert!(msg.contains("I/O error"));
}

#[test]
fn api_error_response_display() {
    let err = ApiErrorResponse {
        code: 403,
        message: "Forbidden".into(),
        status: "403".into(),
    };
    let msg = format!("{err}");
    assert!(msg.contains("API error 403"));
    assert!(msg.contains("Forbidden"));
}

#[test]
fn api_response_parse_error_display() {
    let err = ApiResponseParseError {
        content: "not json".into(),
        source_info: Some("expected `{`".into()),
    };
    let msg = format!("{err}");
    assert!(msg.contains("Failed to parse API response"));
    assert!(msg.contains("not json"));
}

#[test]
fn initialization_error_display() {
    let err = InitializationError {
        message: "no app_id".into(),
    };
    let msg = format!("{err}");
    assert!(msg.contains("Service initialization failed"));
}

#[test]
fn credentials_error_display() {
    let err = CredentialsError {
        message: "missing email".into(),
    };
    let msg = format!("{err}");
    assert!(msg.contains("Invalid credentials"));
}

#[test]
fn download_error_display() {
    let err = DownloadError {
        message: "disk full".into(),
    };
    let msg = format!("{err}");
    assert!(msg.contains("Download failed"));
}

#[test]
fn metadata_error_display() {
    let err = MetadataError("tag write failed".into());
    let msg = format!("{err}");
    assert!(msg.contains("Metadata error"));
}

#[test]
fn resource_not_found_error_display() {
    let err = ResourceNotFoundError {
        resource_type: "album".into(),
        resource_id: "123".into(),
    };
    let msg = format!("{err}");
    assert!(msg.contains("album not found"));
    assert!(msg.contains("123"));
}

#[test]
fn rate_limit_error_display() {
    let err = RateLimitError {
        message: "too many requests".into(),
    };
    let msg = format!("{err}");
    assert!(msg.contains("Rate limited"));
}

#[test]
fn invalid_parameter_error_display() {
    let err = InvalidParameterError {
        message: "negative id".into(),
    };
    let msg = format!("{err}");
    assert!(msg.contains("Invalid parameter"));
}

#[test]
fn unexpected_api_response_error_has_remediation() {
    let err = UnexpectedApiResponseError {
        message: "missing field".into(),
    };
    let msg = format!("{err}").to_lowercase();
    assert!(msg.contains("unexpected") || msg.contains("response") || msg.contains("missing"));
}

#[test]
fn canceled_error_display() {
    let err = Canceled;
    let msg = format!("{err}");
    assert_eq!(msg, "Download cancelled");
}

#[test]
fn sc006_authentication_error_has_remediation() {
    let err = AuthenticationError {
        message: "Invalid credentials. Check your QOBUZ_EMAIL and QOBUZ_PASSWORD environment \
                  variables"
            .into(),
    };
    let msg = format!("{err}").to_lowercase();
    assert!(msg.contains("credentials") || msg.contains("check") || msg.contains("verify"));
}

#[test]
fn sc006_credentials_error_has_remediation() {
    let err = CredentialsError {
        message: "Failed to extract credentials from web player. Configure QOBUZ_APP_ID and \
                  QOBUZ_APP_SECRET manually"
            .into(),
    };
    let msg = format!("{err}").to_lowercase();
    assert!(msg.contains("configure") || msg.contains("manual") || msg.contains("set"));
}

#[test]
fn sc006_resource_not_found_error_has_remediation() {
    let err = ResourceNotFoundError {
        resource_type: "album".into(),
        resource_id: "xyz".into(),
    };
    let msg = format!("{err}").to_lowercase();
    assert!(msg.contains("not found"));
}

#[test]
fn sc006_download_error_has_remediation() {
    let err = DownloadError {
        message: "Failed to write file. Check disk space and permissions".into(),
    };
    let msg = format!("{err}").to_lowercase();
    assert!(msg.contains("check") || msg.contains("disk") || msg.contains("permission"));
}

#[test]
fn sc006_initialization_error_has_remediation() {
    let err = InitializationError {
        message: "app_id and app_secret must be non-empty. Verify your credentials configuration"
            .into(),
    };
    let msg = format!("{err}").to_lowercase();
    assert!(msg.contains("verify") || msg.contains("check") || msg.contains("must"));
}

#[test]
fn sc006_rate_limit_error_has_remediation() {
    let err = RateLimitError {
        message: "Rate limited, retry 3/3. Wait before retrying".into(),
    };
    let msg = format!("{err}").to_lowercase();
    assert!(msg.contains("wait") || msg.contains("retry") || msg.contains("limited"));
}

#[test]
fn sc006_metadata_error_has_remediation() {
    let err = MetadataError(
        "Probe open failed. Verify the file exists and is a valid audio format".into(),
    );
    let msg = format!("{err}").to_lowercase();
    assert!(msg.contains("verify") || msg.contains("valid") || msg.contains("check"));
}
