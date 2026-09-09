//! Central API service holding authentication state and providing all operations.

mod debug;

use std::{env::VarError, path::Path};

use {tokio::runtime::Runtime, tracing::info};

use crate::{
    api::{
        auth::{
            authenticate_with_env, authenticate_with_env_from, login, login_with_token,
            refresh_app_credentials,
        },
        content::{
            albums::{get_album, search_albums},
            artists::{get_artist, get_release_list, search_artists},
            catalog::search_catalog,
            playlists::{get_playlist, search_playlists},
            tracks::{get_track, search_tracks},
        },
        favorites::{
            add_user_favorites, delete_user_favorites, get_user_favorite_ids, get_user_favorites,
        },
        http_client::{HttpClient, ReqwestClient},
    },
    credentials::{load_app_credentials, save_app_credentials, web::extract_from_web_player},
    errors::QobuzApiError::{self, AuthenticationError, InitializationError},
    models::{
        album::Album,
        artist::Artist,
        playlist::Playlist,
        search::{ItemSearchResult, SearchResult, UserFavorites},
        track::Track,
    },
};

/// Base URL for all Qobuz API v0.2 endpoints.
const BASE_URL: &str = "https://www.qobuz.com/api.json/0.2";

/// Central service for all Qobuz API operations.
///
/// Holds authentication state and a shared HTTP client with connection pooling.
pub struct QobuzApiService {
    /// API base URL for requests.
    base_url: String,
    /// Qobuz application ID.
    pub app_id: String,
    /// Qobuz application secret.
    pub app_secret: String,
    /// User authentication token (set after login).
    user_auth_token: Option<String>,
    /// HTTP client with connection pooling.
    client: Box<dyn HttpClient>,
    /// Whether credentials have been refreshed this session.
    credentials_refreshed: bool,
}

impl QobuzApiService {
    delegate!(pub fn search_catalog(query: &str, limit: Option<i32>, offset: Option<i32>) -> SearchResult = search_catalog);

    delegate!(pub fn search_albums(query: &str, limit: Option<i32>, offset: Option<i32>) -> ItemSearchResult<Box<Album>> = search_albums);

    delegate!(pub fn search_artists(query: &str, limit: Option<i32>, offset: Option<i32>) -> ItemSearchResult<Box<Artist>> = search_artists);

    delegate!(pub fn search_tracks(query: &str, limit: Option<i32>, offset: Option<i32>) -> ItemSearchResult<Box<Track>> = search_tracks);

    delegate!(pub fn search_playlists(query: &str, limit: Option<i32>, offset: Option<i32>) -> ItemSearchResult<Box<Playlist>> = search_playlists);

    delegate!(pub fn get_album(album_id: &str, extra: Option<&str>) -> Album = get_album);

    delegate!(pub fn get_artist(artist_id: i32, extra: Option<&str>) -> Artist = get_artist);

    delegate!(pub fn get_track(track_id: i32) -> Track = get_track);

    delegate!(pub fn get_playlist(playlist_id: &str, extra: Option<&str>) -> Playlist = get_playlist);

    delegate!(pub fn get_release_list(artist_id: i32, limit: Option<i32>, offset: Option<i32>) -> ItemSearchResult<Box<Album>> = get_release_list);

    delegate!(pub fn add_user_favorites(item_ids: &[i32], item_type: &str) -> () = add_user_favorites);

    delegate!(pub fn delete_user_favorites(item_ids: &[i32], item_type: &str) -> () = delete_user_favorites);

    delegate!(pub fn get_user_favorites(item_type: &str, limit: Option<i32>, offset: Option<i32>) -> UserFavorites = get_user_favorites);

    delegate!(pub fn get_user_favorite_ids() -> UserFavorites = get_user_favorite_ids);

    /// Builds a service with the given credentials and a new HTTP client.
    ///
    /// # Arguments
    ///
    /// * `app_id` - Qobuz application ID
    /// * `app_secret` - Qobuz application secret
    ///
    /// # Returns
    ///
    /// A `QobuzApiService` with the provided credentials.
    ///
    /// # Errors
    ///
    /// Returns a `QobuzApiError` if `app_id` or `app_secret` is empty, or HTTP client creation
    /// fails.
    fn build_service(app_id: String, app_secret: String) -> Result<Self, QobuzApiError> {
        if app_id.is_empty() || app_secret.is_empty() {
            return Err(InitializationError {
                message: "app_id and app_secret must be non-empty".to_string(),
            });
        }

        let client = ReqwestClient::new(&app_id)?;
        Ok(Self {
            base_url: BASE_URL.to_string(),
            app_id,
            app_secret,
            user_auth_token: None,
            client: Box::new(client),
            credentials_refreshed: false,
        })
    }

    /// Creates a new service instance.
    ///
    /// Extracts app credentials from `.env` or the Qobuz web player JS bundle.
    /// Returns a service with valid `app_id` and `app_secret`.
    /// `user_auth_token` is `None` (unauthenticated state).
    ///
    /// # Returns
    ///
    /// A `QobuzApiService` with valid credentials.
    ///
    /// # Errors
    ///
    /// Returns a `QobuzApiError` if credential extraction or HTTP client creation fails.
    pub fn new() -> Result<Self, QobuzApiError> {
        let env_path = Path::new(".env");
        let (app_id, app_secret) = if let Some(creds) = load_app_credentials(env_path)? {
            creds
        } else {
            info!("No .env credentials found, extracting from web player");

            let rt = Runtime::new()?;
            let (id, secret) = rt.block_on(extract_from_web_player())?;

            save_app_credentials(env_path, &id, &secret)?;

            (id, secret)
        };

        Self::build_service(app_id, app_secret)
    }

    /// Creates a service with explicitly provided app credentials.
    ///
    /// # Arguments
    ///
    /// * `app_id` - Qobuz application ID
    /// * `app_secret` - Qobuz application secret
    ///
    /// # Returns
    ///
    /// A `QobuzApiService` with the provided credentials.
    ///
    /// # Errors
    ///
    /// Returns a `QobuzApiError` if `app_id` or `app_secret` is empty, or HTTP client creation
    /// fails.
    pub fn with_credentials(app_id: &str, app_secret: &str) -> Result<Self, QobuzApiError> {
        Self::build_service(app_id.to_string(), app_secret.to_string())
    }

    /// Returns the API base URL.
    ///
    /// # Returns
    ///
    /// A string slice of the base URL.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Returns the user auth token or an error if not authenticated.
    ///
    /// # Returns
    ///
    /// A string slice of the auth token.
    ///
    /// # Errors
    ///
    /// Returns a `QobuzApiError::AuthenticationError` if not yet authenticated.
    pub fn require_auth_token(&self) -> Result<&str, QobuzApiError> {
        self.user_auth_token
            .as_deref()
            .ok_or_else(|| AuthenticationError {
                message: "Not authenticated. Call authenticate_with_env() or login() first."
                    .to_string(),
            })
    }

    /// Sets the user auth token.
    ///
    /// # Arguments
    ///
    /// * `token` - The authentication token to set
    pub fn set_auth_token(&mut self, token: String) {
        self.user_auth_token = Some(token);
    }

    /// Returns a reference to the HTTP client.
    ///
    /// # Returns
    ///
    /// A `&dyn HttpClient` reference.
    #[must_use]
    pub fn http_client(&self) -> &dyn HttpClient {
        &*self.client
    }

    /// Returns whether credentials have been refreshed this session.
    ///
    /// # Returns
    ///
    /// `true` if credentials have been refreshed.
    #[must_use]
    pub const fn is_credentials_refreshed(&self) -> bool {
        self.credentials_refreshed
    }

    /// Marks credentials as refreshed.
    pub const fn mark_credentials_refreshed(&mut self) {
        self.credentials_refreshed = true;
    }

    /// Returns the app secret.
    ///
    /// # Returns
    ///
    /// A string slice of the app secret.
    #[must_use]
    pub fn app_secret(&self) -> &str {
        &self.app_secret
    }

    /// Rebuilds the HTTP client after updating `app_id`.
    ///
    /// Use this after changing `app_id` to update the `x-app-id` default header.
    ///
    /// # Returns
    ///
    /// A `Result` indicating success or failure.
    ///
    /// # Errors
    ///
    /// Returns a `QobuzApiError` if HTTP client creation fails.
    pub fn rebuild_http_client(&mut self) -> Result<(), QobuzApiError> {
        self.client = ReqwestClient::new(&self.app_id)?.into_boxed();
        Ok(())
    }

    /// Returns a cloned `Box<dyn HttpClient>` for use in spawned tasks.
    ///
    /// # Returns
    ///
    /// A boxed HTTP client suitable for use in async tasks.
    ///
    /// # Errors
    ///
    /// Returns a `QobuzApiError` if HTTP client creation fails.
    pub fn http_client_ref(app_id: &str) -> Result<Box<dyn HttpClient>, QobuzApiError> {
        let client = ReqwestClient::new(app_id)?;
        Ok(client.into_boxed())
    }

    /// Authenticates using environment variables.
    ///
    /// Delegates to [`crate::api::auth::authenticate_with_env`].
    ///
    /// # Returns
    ///
    /// `Ok(())` on successful authentication.
    ///
    /// # Errors
    ///
    /// Returns a `QobuzApiError` if no valid credentials are found or login fails.
    pub fn authenticate_with_env(&mut self) -> Result<(), QobuzApiError> {
        authenticate_with_env(self)
    }

    /// Authenticates using a custom environment variable reader.
    ///
    /// Useful when credentials are stored in a parsed `.env` map rather than
    /// process environment variables.
    ///
    /// # Arguments
    ///
    /// * `get_env` - Function that retrieves environment variable values by name
    ///
    /// # Returns
    ///
    /// `Ok(())` on successful authentication.
    ///
    /// # Errors
    ///
    /// Returns a `QobuzApiError` if no valid credentials are found or login fails.
    pub fn authenticate_with_env_from<E>(&mut self, get_env: E) -> Result<(), QobuzApiError>
    where
        E: Fn(&str) -> Result<String, VarError>,
    {
        authenticate_with_env_from(self, get_env)
    }

    /// Authenticates with email and MD5-hashed password.
    ///
    /// Delegates to [`crate::api::auth::login`].
    ///
    /// # Arguments
    ///
    /// * `email` - User email address
    /// * `password` - User password
    ///
    /// # Returns
    ///
    /// `Ok(())` on successful authentication.
    ///
    /// # Errors
    ///
    /// Returns a `QobuzApiError` if the login request fails.
    pub fn login(&mut self, email: &str, password: &str) -> Result<(), QobuzApiError> {
        login(self, email, password)
    }

    /// Authenticates with user ID and auth token.
    ///
    /// Delegates to [`crate::api::auth::login_with_token`].
    ///
    /// # Arguments
    ///
    /// * `user_id` - Qobuz user ID
    /// * `auth_token` - User authentication token
    ///
    /// # Returns
    ///
    /// `Ok(())` on successful authentication.
    ///
    /// # Errors
    ///
    /// Returns a `QobuzApiError` if the token validation request fails.
    pub fn login_with_token(
        &mut self,
        user_id: &str,
        auth_token: &str,
    ) -> Result<(), QobuzApiError> {
        login_with_token(self, user_id, auth_token)
    }

    /// Re-extracts app credentials from the Qobuz web player.
    ///
    /// Delegates to [`crate::api::auth::refresh_app_credentials`].
    ///
    /// # Returns
    ///
    /// `Ok(())` on successful credential refresh.
    ///
    /// # Errors
    ///
    /// Returns a `QobuzApiError` if credentials have already been refreshed or extraction fails.
    pub fn refresh_app_credentials(&mut self) -> Result<(), QobuzApiError> {
        refresh_app_credentials(self)
    }
}

#[cfg(test)]
impl QobuzApiService {
    /// Creates a test service with a mock HTTP client.
    ///
    /// # Arguments
    ///
    /// * `client` - Boxed HTTP client for test requests.
    /// * `base_url` - Base URL pointing at the mock server.
    ///
    /// # Returns
    ///
    /// A `QobuzApiService` configured for tests.
    #[must_use]
    pub fn new_test(client: Box<dyn HttpClient>, base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            app_id: "test-app-id".to_string(),
            app_secret: "test-app-secret".to_string(),
            user_auth_token: None,
            client,
            credentials_refreshed: false,
        }
    }
}
