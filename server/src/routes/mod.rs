pub mod account;
pub mod alert_rules;
pub mod alert_targets;
pub mod analytics;
pub mod api_keys;
pub mod attachments;
pub mod auth;
pub mod brew_events;
pub mod brews;
pub mod firmware;
pub mod hydrometers;
pub mod readings;
pub mod tosna;
pub mod yeast;

use rocket::http::Status;
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::services::brew_service;

/// Load a brew the caller owns, or answer 404. Every route that takes a brew id
/// (directly or as a parent of events, attachments, analytics...) goes through
/// this so another user's brew is indistinguishable from a missing one.
pub async fn require_brew(
    db: &DatabaseConnection,
    owner: Uuid,
    brew_id: Uuid,
) -> Result<crate::models::entities::brews::Model, Status> {
    match brew_service::find_owned(db, owner, brew_id).await {
        Ok(Some(b)) => Ok(b),
        Ok(None) => Err(Status::NotFound),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[cfg(test)]
mod ownership_tests;
