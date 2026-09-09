//! Integration tests for authentication against the live Qobuz API.
//!
//! These tests read real credentials from a `.env` file and verify that
//! authentication actually works against the Qobuz API.
//!
//! **Tests FAIL if credentials are missing or wrong.** There is no silent skip.
//!
//! Setup: copy `.env.example` to `.env` and fill in your credentials, then:
//!
//! Run with `cargo test --test live --features live-tests`.
//!
//! In CI without credentials, run `cargo test` to run only unit tests and the mock integration.

use anyhow::{Result, anyhow, ensure};

use qobuz_api::api::service::QobuzApiService;

use crate::test_support::ensure_env_credentials;

/// User credentials loaded from the `.env` file.
struct UserCredentials {
    /// Email address or username.
    email: Option<String>,
    /// Password.
    password: Option<String>,
    /// User ID for token-based authentication.
    user_id: Option<String>,
    /// User auth token for token-based authentication.
    user_auth_token: Option<String>,
}

/// Loads and validates user credentials from the environment.
///
/// # Returns
///
/// `Ok(UserCredentials)` with the loaded credentials.
fn require_user_credentials() -> Result<UserCredentials> {
    let env_map = ensure_env_credentials()?;

    Ok(UserCredentials {
        email: env_map
            .get("QOBUZ_EMAIL")
            .or_else(|| env_map.get("QOBUZ_USERNAME"))
            .cloned(),
        password: env_map.get("QOBUZ_PASSWORD").cloned(),
        user_id: env_map.get("QOBUZ_USER_ID").cloned(),
        user_auth_token: env_map.get("QOBUZ_USER_AUTH_TOKEN").cloned(),
    })
}

/// Ensures user credentials exist in the environment without loading them.
///
/// # Errors
///
/// Returns an error if no valid credentials are found.
fn validate_credentials() -> Result<()> {
    let credentials = require_user_credentials()?;
    ensure!(
        credentials.email.is_some() || credentials.user_id.is_some(),
        "no usable credentials found"
    );
    Ok(())
}

/// Creates a `QobuzApiService` using `new()`, which auto-extracts app credentials from the web
/// player (or reads them from `.env` if present).
///
/// # Returns
///
/// A `Result` containing the `QobuzApiService` instance, or an error if creation fails.
fn create_service() -> Result<QobuzApiService> {
    QobuzApiService::new().map_err(|e| anyhow!("Failed to create service: {e}"))
}

#[cfg(test)]
mod tests {
    use {
        anyhow::{Result, anyhow, bail, ensure},
        tracing::info,
    };

    use qobuz_api::errors::QobuzApiError::{ApiErrorResponse, AuthenticationError};

    use crate::auth_tests::{create_service, require_user_credentials, validate_credentials};

    #[test]
    fn live_email_password_login_succeeds() -> Result<()> {
        let creds = require_user_credentials()?;

        let (Some(email), Some(password)) = (&creds.email, &creds.password) else {
            bail!(
                "This test requires QOBUZ_EMAIL (or QOBUZ_USERNAME) and QOBUZ_PASSWORD in \
                 .env.\nAlternatively, this test is not applicable to token-only auth setups."
            );
        };

        let mut service = create_service()?;

        service.login(email, password)?;

        let token = service.require_auth_token().map_err(|original| {
            anyhow!("Login succeeded but no auth token was stored: {original}")
        })?;
        ensure!(!token.is_empty(), "Auth token should not be empty");

        info!("email/password login succeeded against live API");
        Ok(())
    }

    #[test]
    fn live_email_password_wrong_credentials_fail() -> Result<()> {
        validate_credentials()?;

        let mut service = create_service()?;

        let result = service.login("nonexistent@example.com", "wrongpassword123");

        ensure!(result.is_err(), "Login with wrong credentials should fail");

        let Err(err) = result else {
            bail!("Expected error for wrong credentials");
        };

        ensure!(
            matches!(err, ApiErrorResponse { .. } | AuthenticationError { .. }),
            "Wrong credentials should return ApiErrorResponse or AuthenticationError, got: {err:?}"
        );

        info!("wrong credentials correctly rejected by live API");
        Ok(())
    }

    #[test]
    fn live_token_login_succeeds() -> Result<()> {
        let creds = require_user_credentials()?;

        let (Some(user_id), Some(auth_token)) = (&creds.user_id, &creds.user_auth_token) else {
            bail!(
                "This test requires QOBUZ_USER_ID and QOBUZ_USER_AUTH_TOKEN in \
                 .env.\nAlternatively, this test is not applicable to email/password-only auth \
                 setups."
            );
        };

        let mut service = create_service()?;

        service.login_with_token(user_id, auth_token)?;

        let stored_token = service.require_auth_token().map_err(|original| {
            anyhow!("Token login succeeded but no auth token was stored: {original}")
        })?;
        ensure!(!stored_token.is_empty(), "Auth token should not be empty");

        info!("token login succeeded against live API");
        Ok(())
    }

    #[test]
    fn live_env_auth_succeeds() -> Result<()> {
        validate_credentials()?;

        let mut service = create_service()?;
        service.authenticate_with_env()?;

        let token = service.require_auth_token().map_err(|original| {
            anyhow!("Env auth succeeded but no auth token was stored: {original}")
        })?;
        ensure!(!token.is_empty(), "Auth token should not be empty");

        info!("env-based authentication succeeded against live API");
        Ok(())
    }

    #[test]
    fn live_service_new_reads_env_and_authenticates() -> Result<()> {
        validate_credentials()?;

        let mut service = create_service()?;
        service.authenticate_with_env()?;

        let token = service.require_auth_token()?;
        ensure!(!token.is_empty(), "Auth token should not be empty");

        info!("full service::new() + authenticate_with_env() succeeded against live API");
        Ok(())
    }
}
