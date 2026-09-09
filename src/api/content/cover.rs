//! Cover art fetching for tracks.

use tracing::warn;

use crate::{api::service::QobuzApiService, metadata::extractor::ComprehensiveMetadata};

/// Fetches cover art binary data for a track if a cover art URL is available.
///
/// # Arguments
///
/// * `service` - Authenticated API service
/// * `meta` - Comprehensive metadata containing the cover art URL
/// * `token` - User authentication token
///
/// # Returns
///
/// Cover art image data as bytes, or `None` if unavailable.
pub async fn fetch_track_cover(
    service: &QobuzApiService,
    meta: &ComprehensiveMetadata,
    token: &str,
) -> Option<Vec<u8>> {
    let url = meta.cover_art_url.as_deref()?;
    let resp = match service.http_client().get_with_auth(url, token, None).await {
        Ok(r) => r,
        Err(e) => {
            warn!(error = %e, "Cover art HTTP request failed");
            return None;
        }
    };
    match resp.bytes().await {
        Err(e) => {
            warn!(error = %e, "Failed to read cover art bytes");
            None
        }
        Ok(b) => Some(b.to_vec()),
    }
}
