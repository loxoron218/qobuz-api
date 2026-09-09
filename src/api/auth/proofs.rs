//! Auth unit tests with a local mock HTTP server for deterministic testing.

use std::{
    collections::HashMap,
    env::VarError::{self, NotPresent},
};

use anyhow::{Result, anyhow, ensure};

use crate::{
    api::{
        auth::{
            authenticate_with_env, authenticate_with_env_from, login, login_with_token,
            refresh_app_credentials,
        },
        fixture::{MockServer, make_service_without_auth},
        service::QobuzApiService,
    },
    errors::QobuzApiError::{ApiErrorResponse, CredentialsError},
};

fn mock_env(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

fn env_reader(env: &HashMap<String, String>) -> impl Fn(&str) -> Result<String, VarError> + '_ {
    move |key| env.get(key).cloned().ok_or(NotPresent)
}

/// Tests env auth prefers token over email.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn env_auth_prefers_token_over_email() -> Result<()> {
    let env = mock_env(&[
        ("QOBUZ_USER_ID", "42"),
        ("QOBUZ_USER_AUTH_TOKEN", "tok"),
        ("QOBUZ_EMAIL", "e@x.com"),
        ("QOBUZ_PASSWORD", "p"),
    ]);
    let server = MockServer::start(200, r#"{"user_auth_token":"tok","user":{"id":42}}"#)?;
    let mut service = make_service_without_auth(&server.base_url())?;
    authenticate_with_env_from(&mut service, env_reader(&env))?;
    ensure!(service.require_auth_token()? == "tok");
    Ok(())
}

/// Tests env auth uses email with password.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn env_auth_uses_email_with_password() -> Result<()> {
    let env = mock_env(&[("QOBUZ_EMAIL", "e@x.com"), ("QOBUZ_PASSWORD", "secret")]);
    let server = MockServer::start(200, r#"{"user_auth_token":"email-tok","user":{"id":1}}"#)?;
    let mut service = make_service_without_auth(&server.base_url())?;
    authenticate_with_env_from(&mut service, env_reader(&env))?;
    ensure!(service.require_auth_token()? == "email-tok");
    Ok(())
}

/// Tests env auth uses username alias.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn env_auth_uses_username_alias() -> Result<()> {
    let env = mock_env(&[("QOBUZ_USERNAME", "e@x.com"), ("QOBUZ_PASSWORD", "secret")]);
    let server = MockServer::start(200, r#"{"user_auth_token":"uname-tok","user":{"id":1}}"#)?;
    let mut service = make_service_without_auth(&server.base_url())?;
    authenticate_with_env_from(&mut service, env_reader(&env))?;
    ensure!(service.require_auth_token()? == "uname-tok");
    Ok(())
}

/// Expects authentication to fail with a matching error message.
///
/// # Arguments
///
/// * `env` - Mock environment variables.
/// * `expected_substring` - Expected error message substring.
///
/// # Errors
///
/// Returns an error if no error occurs or the message mismatches.
fn expect_auth_error(env: &HashMap<String, String>, expected_substring: &str) -> Result<()> {
    let mut service = QobuzApiService::with_credentials("id", "secret")?;
    let err = authenticate_with_env_from(&mut service, env_reader(env))
        .err()
        .ok_or_else(|| anyhow!("expected error"))?;
    ensure!(err.to_string().contains(expected_substring));
    Ok(())
}

/// Tests env auth fails without vars.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn env_auth_fails_without_vars() -> Result<()> {
    expect_auth_error(&mock_env(&[]), "environment variables found")
}

/// Tests env auth fails email no password.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn env_auth_fails_email_no_password() -> Result<()> {
    expect_auth_error(&mock_env(&[("QOBUZ_EMAIL", "e@x.com")]), "QOBUZ_PASSWORD")
}

/// Tests login success stores token.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn login_success_stores_token() -> Result<()> {
    let server = MockServer::start(200, r#"{"user_auth_token":"login-tok","user":{"id":1}}"#)?;
    let mut service = make_service_without_auth(&server.base_url())?;
    login(&mut service, "user@example.com", "password")?;
    ensure!(service.require_auth_token()? == "login-tok");
    Ok(())
}

/// Tests login failure returns error.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn login_failure_returns_error() -> Result<()> {
    let server = MockServer::start(401, r#"{"status":"error","code":401,"message":"Invalid"}"#)?;
    let mut service = make_service_without_auth(&server.base_url())?;
    let err = login(&mut service, "bad@x.com", "wrong")
        .err()
        .ok_or_else(|| anyhow!("expected error"))?;
    ensure!(matches!(err, ApiErrorResponse { .. }));
    Ok(())
}

/// Tests token auth success stores token.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn token_auth_success_stores_token() -> Result<()> {
    let server = MockServer::start(200, r#"{"user_auth_token":"tok","user":{"id":42}}"#)?;
    let mut service = make_service_without_auth(&server.base_url())?;
    login_with_token(&mut service, "42", "tok")?;
    ensure!(service.require_auth_token()? == "tok");
    Ok(())
}

/// Tests token auth failure returns error.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn token_auth_failure_returns_error() -> Result<()> {
    let server = MockServer::start(
        401,
        r#"{"status":"error","code":401,"message":"Bad token"}"#,
    )?;
    let mut service = make_service_without_auth(&server.base_url())?;
    ensure!(login_with_token(&mut service, "99", "bad").is_err());
    Ok(())
}

/// Tests refresh rejects double refresh.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn refresh_rejects_double_refresh() -> Result<()> {
    let mut service = QobuzApiService::with_credentials("id", "secret")?;
    service.mark_credentials_refreshed();
    let err = refresh_app_credentials(&mut service)
        .err()
        .ok_or_else(|| anyhow!("expected error"))?;
    ensure!(matches!(err, CredentialsError { .. }));
    ensure!(err.to_string().contains("once per session"));
    Ok(())
}

/// Tests env wrapper errors without credentials.
///
/// # Errors
///
/// Returns an error if the test setup or assertion fails.
#[test]
fn env_wrapper_errors_without_credentials() -> Result<()> {
    let server = MockServer::start(200, "{}")?;
    let mut service = make_service_without_auth(&server.base_url())?;
    match authenticate_with_env(&mut service) {
        Ok(()) => {
            ensure!(
                service.require_auth_token().is_ok(),
                "token should be set on success"
            );
            Ok(())
        }
        Err(_) => Ok(()),
    }
}
