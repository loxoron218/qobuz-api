//! User authentication credentials.

use serde::{Deserialize, Serialize};

/// User authentication credentials for the Qobuz API.
///
/// All fields are optional to accommodate different authentication methods.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Credential {
    /// Qobuz user ID.
    pub user_id: Option<String>,
    /// Authentication token.
    pub user_auth_token: Option<String>,
    /// Email address.
    pub email: Option<String>,
    /// MD5-hashed password.
    pub password: Option<String>,
    /// Application ID.
    pub app_id: Option<String>,
    /// Application secret.
    pub app_secret: Option<String>,
}
