//! Authentication methods for the Qobuz API.

#[cfg(test)]
pub mod proofs;

use std::env::VarError;

use crate::{api::service_state::QobuzApiService, errors::QobuzApiError};

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
    service.authenticate_with_env()
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
    service.authenticate_with_env_from(get_env)
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
    service.login(email, password)
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
    service.login_with_token(user_id, auth_token)
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
    service.refresh_app_credentials()
}
