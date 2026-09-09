//! Authentication methods for the Qobuz API.

use std::{
    env::{VarError, var},
    path::Path,
};

use {
    md5::{Digest, Md5},
    serde::Deserialize,
    tokio::runtime::Runtime,
    tracing::{error, info},
};

use crate::{
    api::{http_client::HttpClient, requests::post, service::QobuzApiService},
    credentials::{save_app_credentials, web::extract_from_web_player},
    errors::QobuzApiError::{self, AuthenticationError, CredentialsError},
    signing::to_hex,
};

/// Response from the Qobuz login endpoint.
#[derive(Deserialize)]
struct LoginResponse {
    /// User authentication token returned on successful login.
    user_auth_token: Option<String>,
    /// User object returned on successful login.
    user: Option<LoginUser>,
}

/// User data within the login response.
#[derive(Deserialize)]
struct LoginUser {
    /// Numeric user ID.
    id: i64,
}

/// Authenticates using environment variables.
///
/// Reads credentials in priority order:
/// 1. `QOBUZ_USER_ID` + `QOBUZ_USER_AUTH_TOKEN` → token-based auth
/// 2. `QOBUZ_EMAIL` + `QOBUZ_PASSWORD` → email/password auth
/// 3. `QOBUZ_USERNAME` + `QOBUZ_PASSWORD` → username/password auth
///
/// # Arguments
///
/// * `service` - Mutable reference to the API service to authenticate
///
/// # Returns
///
/// `Ok(())` on successful authentication.
///
/// # Errors
///
/// Returns a `QobuzApiError` if no valid credentials are found or the login request fails.
pub fn authenticate_with_env(service: &mut QobuzApiService) -> Result<(), QobuzApiError> {
    authenticate_with_env_from(service, |key| var(key))
}

/// Env-abstracted authentication for deterministic testing without `set_var`.
///
/// # Arguments
///
/// * `service` - Mutable reference to the API service to authenticate
/// * `get_env` - Function that retrieves environment variable values by name
///
/// # Returns
///
/// `Ok(())` on successful authentication.
///
/// # Errors
///
/// Returns `QobuzApiError` if no valid credentials are found or login fails.
pub fn authenticate_with_env_from<E>(
    service: &mut QobuzApiService,
    get_env: E,
) -> Result<(), QobuzApiError>
where
    E: Fn(&str) -> Result<String, VarError>,
{
    if let (Ok(user_id), Ok(token)) = (get_env("QOBUZ_USER_ID"), get_env("QOBUZ_USER_AUTH_TOKEN")) {
        info!("Using token-based authentication from environment");

        let rt = Runtime::new()?;
        rt.block_on(login_with_token_inner(
            service.http_client(),
            service.base_url(),
            &service.app_id,
            user_id.trim(),
            token.trim(),
        ))?;

        service.set_auth_token(token.trim().to_string());
        info!(method = "token", "Authentication successful");
        return Ok(());
    }

    let email = get_env("QOBUZ_EMAIL")
        .or_else(|_| get_env("QOBUZ_USERNAME"))
        .map_err(|var_error| AuthenticationError {
            message: format!(
                "No QOBUZ_USER_ID/QOBUZ_USER_AUTH_TOKEN or QOBUZ_EMAIL/QOBUZ_PASSWORD environment \
                 variables found: {var_error}"
            ),
        })?;

    let password = get_env("QOBUZ_PASSWORD").map_err(|var_error| AuthenticationError {
        message: format!("QOBUZ_PASSWORD environment variable not found: {var_error}"),
    })?;

    info!("Using email/password authentication from environment");

    let rt = Runtime::new()?;
    let token = rt.block_on(login_inner(
        service.http_client(),
        service.base_url(),
        &service.app_id,
        email.trim(),
        password.trim(),
    ))?;

    service.set_auth_token(token);
    info!(method = "email", "Authentication successful");
    Ok(())
}

/// Authenticates with email and MD5-hashed password via `POST /user/login`.
///
/// # Arguments
///
/// * `service` - Mutable reference to the API service to authenticate
/// * `email` - User email address
/// * `password` - User password (will be MD5-hashed before sending)
///
/// # Returns
///
/// `Ok(())` on successful authentication.
///
/// # Errors
///
/// Returns a `QobuzApiError` if the login request fails or no token is returned.
pub fn login(
    service: &mut QobuzApiService,
    email: &str,
    password: &str,
) -> Result<(), QobuzApiError> {
    info!(email = email, "Attempting login");

    let rt = Runtime::new()?;
    let token = rt.block_on(login_inner(
        service.http_client(),
        service.base_url(),
        &service.app_id,
        email,
        password,
    ))?;

    service.set_auth_token(token);
    info!("Login successful");
    Ok(())
}

/// Inner login logic: hashes the password and sends the login POST request.
///
/// # Arguments
///
/// * `client` - HTTP client implementation
/// * `base_url` - API base URL
/// * `app_id` - Application ID
/// * `email` - User email address
/// * `password` - User password (will be MD5-hashed)
///
/// # Returns
///
/// The user authentication token on successful login.
///
/// # Errors
///
/// Returns a `QobuzApiError` if the HTTP request fails or the response contains no token.
async fn login_inner(
    client: &dyn HttpClient,
    base_url: &str,
    app_id: &str,
    email: &str,
    password: &str,
) -> Result<String, QobuzApiError> {
    let hashed = to_hex(&Md5::digest(password.as_bytes()));

    let mut params = vec![
        ("email".to_string(), email.to_string()),
        ("password".to_string(), hashed),
    ];

    let response: LoginResponse =
        post(client, base_url, "/user/login", &mut params, app_id, "").await?;

    response.user_auth_token.ok_or_else(|| {
        let err = AuthenticationError {
            message: "Login succeeded but no user_auth_token in response".to_string(),
        };
        error!(error = %err, "Login response missing user_auth_token");
        err
    })
}

/// Authenticates with user ID and auth token via `POST /user/login`.
///
/// # Arguments
///
/// * `service` - Mutable reference to the API service to authenticate
/// * `user_id` - Qobuz user ID
/// * `auth_token` - User authentication token
///
/// # Returns
///
/// `Ok(())` on successful authentication.
///
/// # Errors
///
/// Returns a `QobuzApiError` if the login request fails or no token is confirmed.
pub fn login_with_token(
    service: &mut QobuzApiService,
    user_id: &str,
    auth_token: &str,
) -> Result<(), QobuzApiError> {
    info!(user_id = user_id, "Attempting token login");

    let rt = Runtime::new()?;
    rt.block_on(login_with_token_inner(
        service.http_client(),
        service.base_url(),
        &service.app_id,
        user_id,
        auth_token,
    ))?;

    service.set_auth_token(auth_token.to_string());
    info!("Token login successful");
    Ok(())
}

/// Inner token login logic: sends the login POST request with user ID and token.
///
/// # Arguments
///
/// * `client` - HTTP client implementation
/// * `base_url` - API base URL
/// * `app_id` - Application ID
/// * `user_id` - Qobuz user ID
/// * `auth_token` - User authentication token
///
/// # Returns
///
/// `Ok(())` on successful token validation.
///
/// # Errors
///
/// Returns a `QobuzApiError` if the HTTP request fails or the response contains no token.
async fn login_with_token_inner(
    client: &dyn HttpClient,
    base_url: &str,
    app_id: &str,
    user_id: &str,
    auth_token: &str,
) -> Result<(), QobuzApiError> {
    let mut params = vec![
        ("user_id".to_string(), user_id.to_string()),
        ("user_auth_token".to_string(), auth_token.to_string()),
    ];

    let response: LoginResponse =
        post(client, base_url, "/user/login", &mut params, app_id, "").await?;

    if response.user_auth_token.is_none() {
        let err = AuthenticationError {
            message: "Token login succeeded but no user_auth_token in response".to_string(),
        };
        error!(error = %err, "Token login response missing user_auth_token");
        return Err(err);
    }

    if let Some(user) = &response.user {
        let returned_id = user.id.to_string();
        if returned_id != user_id {
            let err = AuthenticationError {
                message: format!(
                    "User ID mismatch: requested {user_id} but API returned {returned_id}"
                ),
            };
            error!(error = %err, "Token login user ID mismatch");
            return Err(err);
        }
    }

    Ok(())
}

/// Re-extracts app credentials from the Qobuz web player.
///
/// Can only be called once per session. Returns error on subsequent calls.
///
/// # Arguments
///
/// * `service` - Mutable reference to the API service to refresh
///
/// # Returns
///
/// `Ok(())` on successful credential refresh.
///
/// # Errors
///
/// Returns a `QobuzApiError` if credentials have already been refreshed or extraction fails.
pub fn refresh_app_credentials(service: &mut QobuzApiService) -> Result<(), QobuzApiError> {
    if service.is_credentials_refreshed() {
        let err = CredentialsError {
            message: "Credentials can only be refreshed once per session".to_string(),
        };
        error!(error = %err, "Credential refresh attempted more than once per session");
        return Err(err);
    }

    info!("Refreshing app credentials from web player");

    let rt = Runtime::new()?;
    let (app_id, app_secret) = rt.block_on(extract_from_web_player()).map_err(|e| {
        error!(error = %e, "Web player credential extraction failed; manual configuration required");
        e
    })?;

    let env_path = Path::new(".env");
    save_app_credentials(env_path, &app_id, &app_secret)?;

    service.app_id = app_id;
    service.app_secret = app_secret;
    service.rebuild_http_client()?;
    service.mark_credentials_refreshed();

    info!("App credentials refreshed successfully");
    Ok(())
}

#[cfg(test)]
mod auth_tests;
