//! Debug redaction for the central API service.

use std::fmt::{Debug, Formatter, Result as FmtResult};

use crate::api::service::QobuzApiService;

impl Debug for QobuzApiService {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("QobuzApiService")
            .field("base_url", &self.base_url)
            .field("app_id", &self.app_id)
            .field("app_secret", &"<redacted>")
            .field(
                "user_auth_token",
                &self.user_auth_token.as_deref().map(|_| "<redacted>"),
            )
            .field("credentials_refreshed", &self.credentials_refreshed)
            .finish_non_exhaustive()
    }
}
