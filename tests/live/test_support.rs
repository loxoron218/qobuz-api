//! Shared test helpers for integration tests.

pub mod query;
pub mod setup;

use std::{
    collections::HashMap,
    env::{VarError::NotPresent, var},
    path::Path,
    sync::OnceLock,
};

use {
    anyhow::{Result, anyhow, bail},
    tracing::{Level, warn},
    tracing_subscriber::{EnvFilter, fmt},
};

use qobuz_api::{
    api::service::QobuzApiService, credentials::parse_env_file, models::file_url::quality::MP3_320,
};

/// Test support imports macro for integration tests.
#[macro_export]
macro_rules! test_support_imports {
    () => {
        use {
            anyhow::{Result, anyhow, ensure},
            tracing::info,
        };

        use qobuz_api::models::{album::Album, artist::Artist, playlist::Playlist, track::Track};
    };
}

/// Threshold in seconds for trial duration tests.
pub const TRIAL_DURATION_THRESHOLD_SECS: f64 = 30.0;

/// Parsed `.env` file contents, cached for the lifetime of the process.
static ENV_MAP: OnceLock<HashMap<String, String>> = OnceLock::new();

/// Global download configuration, initialized once from environment variables.
static CONFIG: OnceLock<DownloadConfig> = OnceLock::new();

/// Global logging guard, ensures tracing subscriber is initialized exactly once.
static LOG_GUARD: OnceLock<()> = OnceLock::new();

/// Browse IDs for browse/content-detail tests, initialized once from environment variables.
static BROWSE_IDS: OnceLock<BrowseIds> = OnceLock::new();

/// Test keywords for search tests, initialized once from environment variables.
static TEST_KEYWORDS: OnceLock<TestKeywords> = OnceLock::new();

/// Test data: IDs and queries configurable via `.env`.
#[derive(Debug)]
pub struct BrowseIds {
    /// Album query string.
    pub album: String,
    /// Artist query string.
    pub artist: String,
    /// Track query string.
    pub track: String,
    /// Playlist query string.
    pub playlist: String,
    /// Release list artist query string.
    pub release_list_artist: String,
}

impl BrowseIds {
    /// Creates IDs from environment variables.
    ///
    /// # Returns
    ///
    /// A new `BrowseIds` instance populated from environment.
    fn from_env() -> Self {
        let env = load_env_file();

        Self {
            album: env_var_or(env, "TEST_BROWSE_ALBUM_QUERY", "Kind of Blue Miles Davis"),
            artist: env_var_or(env, "TEST_BROWSE_ARTIST_QUERY", "Miles Davis"),
            track: env_var_or(env, "TEST_BROWSE_TRACK_QUERY", "So What Miles Davis"),
            playlist: env_var_or(env, "TEST_BROWSE_PLAYLIST_QUERY", "jazz classics"),
            release_list_artist: env_var_or(env, "TEST_BROWSE_RELEASE_LIST_QUERY", "John Coltrane"),
        }
    }
}

/// Configuration for download tests.
#[derive(Debug)]
pub struct DownloadConfig {
    /// Query string for searching tracks.
    pub track_query: String,
    /// Query string for searching albums.
    pub album_query: String,
    /// Audio format ID (e.g., 3 for 320kbps MP3).
    pub format_id: i32,
}

impl DownloadConfig {
    /// Creates configuration from environment variables.
    ///
    /// # Returns
    ///
    /// A new `DownloadConfig` instance populated from environment.
    fn from_env() -> Self {
        let env = load_env_file();

        Self {
            track_query: env_var_or(env, "TEST_DOWNLOAD_TRACK_QUERY", "So What Miles Davis"),
            album_query: env_var_or(env, "TEST_DOWNLOAD_ALBUM_QUERY", "Kind of Blue Miles Davis"),
            format_id: env_var_or(env, "TEST_DOWNLOAD_FORMAT_ID", "")
                .parse::<i32>()
                .unwrap_or(MP3_320),
        }
    }

    /// Returns the album query string.
    ///
    /// # Returns
    ///
    /// A string slice containing the album query.
    #[must_use]
    pub fn album_query(&self) -> &str {
        &self.album_query
    }
}

/// Search keywords for integration tests, configurable via `.env`.
#[derive(Debug)]
pub struct TestKeywords {
    /// Album search query #1.
    pub album_query_1: String,
    /// Album search query #2.
    pub album_query_2: String,
    /// Artist search query #1.
    pub artist_query_1: String,
    /// Artist search query #2.
    pub artist_query_2: String,
    /// Track search query #1.
    pub track_query_1: String,
    /// Track search query #2.
    pub track_query_2: String,
    /// Playlist search query #1.
    pub playlist_query_1: String,
    /// Catalog search query.
    pub catalog_query: String,
    /// Pagination search query.
    pub pagination_query: String,
}

impl TestKeywords {
    /// Creates a `TestKeywords` instance from environment variables.
    ///
    /// # Returns
    ///
    /// A `TestKeywords` with values from `.env` or defaults.
    fn from_env() -> Self {
        let env = load_env_file();

        Self {
            album_query_1: env_var_or(
                env,
                "TEST_SEARCH_ALBUM_QUERY_1",
                "The Dark Side of the Moon Pink Floyd",
            ),
            album_query_2: env_var_or(env, "TEST_SEARCH_ALBUM_QUERY_2", "Kind of Blue Miles Davis"),
            artist_query_1: env_var_or(env, "TEST_SEARCH_ARTIST_QUERY_1", "Pink Floyd"),
            artist_query_2: env_var_or(env, "TEST_SEARCH_ARTIST_QUERY_2", "Miles Davis"),
            track_query_1: env_var_or(
                env,
                "TEST_SEARCH_TRACK_QUERY_1",
                "Comfortably Numb Pink Floyd",
            ),
            track_query_2: env_var_or(env, "TEST_SEARCH_TRACK_QUERY_2", "So What Miles Davis"),
            playlist_query_1: env_var_or(env, "TEST_SEARCH_PLAYLIST_QUERY_1", "jazz classics"),
            catalog_query: env_var_or(env, "TEST_SEARCH_CATALOG_QUERY", "0190244849000"),
            pagination_query: env_var_or(env, "TEST_SEARCH_PAGINATION_QUERY", "Pink Floyd"),
        }
    }
}

/// Returns the browse IDs from environment variables.
///
/// # Returns
///
/// A static reference to the browse IDs.
pub fn get_browse_ids() -> &'static BrowseIds {
    BROWSE_IDS.get_or_init(BrowseIds::from_env)
}

/// Returns the test keywords from environment variables.
///
/// # Returns
///
/// A static reference to `TestKeywords` with values from `.env` or defaults.
pub fn get_test_keywords() -> &'static TestKeywords {
    TEST_KEYWORDS.get_or_init(TestKeywords::from_env)
}

/// Returns the download configuration from environment variables.
///
/// # Returns
///
/// A static reference to the download configuration.
pub fn get_download_config() -> &'static DownloadConfig {
    CONFIG.get_or_init(DownloadConfig::from_env)
}

/// Loads and caches environment variables from `.env` file if present.
///
/// # Returns
///
/// A static reference to the cached environment map.
pub fn load_env_file() -> &'static HashMap<String, String> {
    ENV_MAP.get_or_init(|| {
        let env_path = Path::new(".env");
        if !env_path.exists() {
            return HashMap::new();
        }

        match parse_env_file(env_path) {
            Ok(pairs) => pairs.into_iter().collect(),
            Err(e) => {
                warn!(error = %e, "Failed to load .env");
                HashMap::new()
            }
        }
    })
}

/// Reads an environment variable from the cached `.env` file, falling back to
/// process environment variables, then to a default.
fn env_var_or(map: &HashMap<String, String>, key: &str, default: &str) -> String {
    if let Some(value) = map.get(key) {
        return value.clone();
    }
    var(key).unwrap_or_else(|_| default.to_string())
}

/// Loads and caches the `.env` file at `path`, returning an error on I/O failure.
fn load_env_file_at(path: &Path) -> Result<HashMap<String, String>> {
    parse_env_file(path)
        .map(|pairs| pairs.into_iter().collect())
        .map_err(|e| anyhow!("Failed to parse .env: {e}"))
}

/// Initializes logging for tests.
pub fn init_logging() {
    let () = LOG_GUARD.get_or_init(|| {
        fmt()
            .with_env_filter(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
            .init();
    });
}

/// Validates that user credentials exist in the environment.
///
/// Loads the `.env` file and checks for either email/password or user ID/token credentials.
///
/// # Returns
///
/// The parsed `.env` map if valid credentials are found.
///
/// # Errors
///
/// Returns an error if no `.env` file exists or no valid credential pair is found.
pub fn ensure_env_credentials() -> Result<HashMap<String, String>> {
    let env_path = Path::new(".env");

    if !env_path.exists() {
        bail!(
            "No .env file found. Copy .env.example to .env and fill in your Qobuz \
             credentials.\nSet either QOBUZ_EMAIL + QOBUZ_PASSWORD or QOBUZ_USER_ID + \
             QOBUZ_USER_AUTH_TOKEN."
        );
    }

    let env_map = load_env_file_at(env_path)?;

    let email = env_map
        .get("QOBUZ_EMAIL")
        .or_else(|| env_map.get("QOBUZ_USERNAME"))
        .cloned();
    let password = env_map.get("QOBUZ_PASSWORD").cloned();
    let user_id = env_map.get("QOBUZ_USER_ID").cloned();
    let user_auth_token = env_map.get("QOBUZ_USER_AUTH_TOKEN").cloned();

    let has_email_auth = email.is_some() && password.is_some();
    let has_token_auth = user_id.is_some() && user_auth_token.is_some();

    if !has_email_auth && !has_token_auth {
        bail!(
            "No user credentials in .env. Provide one of:\n- QOBUZ_EMAIL (or QOBUZ_USERNAME) + \
             QOBUZ_PASSWORD\n- QOBUZ_USER_ID + QOBUZ_USER_AUTH_TOKEN"
        );
    }

    if email.is_some() && password.is_none() {
        bail!("QOBUZ_EMAIL is set but QOBUZ_PASSWORD is missing in .env");
    }

    Ok(env_map)
}

/// Creates an authenticated Qobuz API service.
///
/// # Returns
///
/// An authenticated service if credentials are valid.
///
/// # Errors
///
/// Returns an error if credentials are missing or authentication fails.
pub fn create_authenticated_service() -> Result<QobuzApiService> {
    let env_map = ensure_env_credentials()?;

    let mut service =
        QobuzApiService::new().map_err(|e| anyhow!("Failed to create service: {e}"))?;
    service
        .authenticate_with_env_from(|key| env_map.get(key).cloned().ok_or(NotPresent))
        .map_err(|e| anyhow!("Authentication failed: {e}"))?;
    Ok(service)
}
