//! Subscription and user data models.

use serde::{Deserialize, Serialize};

use crate::models::credential::Credential;

/// User subscription details.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Subscription {
    /// Subscription ID.
    pub id: Option<i32>,
    /// Subscription plan name.
    pub offer: Option<String>,
    /// Subscription start date.
    pub start_date: Option<String>,
    /// Subscription end date.
    pub end_date: Option<String>,
    /// Subscription status.
    pub status: Option<String>,
    /// Whether the subscription is active.
    pub is_active: Option<bool>,
}

/// A Qobuz user.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct User {
    /// User ID.
    pub id: Option<i32>,
    /// User credentials/capabilities.
    pub credential: Option<Credential>,
    /// Subscription details.
    pub subscription: Option<Subscription>,
    /// Display name.
    pub display_name: Option<String>,
}
