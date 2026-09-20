//! Service delegates for user library (favorites) operations.

use tokio::runtime::Runtime;

use crate::api::{
    content::catalog::UserFavorites,
    favorites::{
        add_user_favorites, delete_user_favorites, get_user_favorite_ids, get_user_favorites,
    },
    service::QobuzApiService,
};

impl QobuzApiService {
    delegate!(
        /// Adds items to the user's favorites.
        ///
        /// # Arguments
        ///
        /// * `item_ids` - IDs of items to favorite
        /// * `item_type` - Type of items (`"album"`, `"artist"`, or `"track"`)
        ///
        /// # Returns
        ///
        /// `Ok(())` on success.
        pub fn add_user_favorites(item_ids: &[i32], item_type: &str) -> () = add_user_favorites
    );

    delegate!(
        /// Removes items from the user's favorites.
        ///
        /// # Arguments
        ///
        /// * `item_ids` - IDs of items to remove
        /// * `item_type` - Type of items (`"album"`, `"artist"`, or `"track"`)
        ///
        /// # Returns
        ///
        /// `Ok(())` on success.
        pub fn delete_user_favorites(item_ids: &[i32], item_type: &str) -> () = delete_user_favorites
    );

    delegate!(
        /// Retrieves the user's favorites list.
        ///
        /// # Arguments
        ///
        /// * `item_type` - Type of items to retrieve
        /// * `limit` - Maximum number of results
        /// * `offset` - Pagination offset
        ///
        /// # Returns
        ///
        /// The user's favorited items grouped by type.
        pub fn get_user_favorites(item_type: &str, limit: Option<i32>, offset: Option<i32>) -> UserFavorites = get_user_favorites
    );

    delegate!(
        /// Retrieves only the favorite IDs grouped by type.
        ///
        /// # Returns
        ///
        /// The user's favorite IDs grouped by type.
        pub fn get_user_favorite_ids() -> UserFavorites = get_user_favorite_ids
    );
}

#[cfg(test)]
mod tests {
    use anyhow::{Result, ensure};

    use crate::api::fixture::{MockServer, make_service};

    #[test]
    fn add_user_favorites_succeeds() -> Result<()> {
        let server = MockServer::start(200, "{}")?;
        let service = make_service(&server.base_url())?;
        service.add_user_favorites(&[1, 2], "track")?;
        Ok(())
    }

    #[test]
    fn delete_user_favorites_succeeds() -> Result<()> {
        let server = MockServer::start(200, "{}")?;
        let service = make_service(&server.base_url())?;
        service.delete_user_favorites(&[1, 2], "track")?;
        Ok(())
    }

    #[test]
    fn get_user_favorites_returns_ids() -> Result<()> {
        let server = MockServer::start(200, r#"{"track_ids":[1,2]}"#)?;
        let service = make_service(&server.base_url())?;
        let favorites = service.get_user_favorites("track", Some(50), None)?;
        ensure!(favorites.track_ids == Some(vec![1, 2]));
        Ok(())
    }

    #[test]
    fn get_user_favorite_ids_returns_ids() -> Result<()> {
        let server = MockServer::start(200, r#"{"track_ids":[7]}"#)?;
        let service = make_service(&server.base_url())?;
        let favorites = service.get_user_favorite_ids()?;
        ensure!(favorites.track_ids == Some(vec![7]));
        Ok(())
    }
}
