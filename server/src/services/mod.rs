pub mod account_service;
pub mod alert_rule_service;
pub mod alert_target_service;
pub mod analytics_service;
pub mod api_keys;
pub mod attachment_service;
pub mod brew_event_service;
pub mod brew_service;
pub mod firmware_service;
pub mod hydrometer_service;
pub mod limits;
pub mod preferences_service;
pub mod reading_service;
pub mod report_service;
pub mod scanner_service;
pub mod sessions;
pub mod tosna_service;
pub mod user_service;
pub mod webhook_dispatcher;
pub mod webhook_guard;

use sea_orm::DbErr;

/// Why a write to an owned resource failed. Shared by services that validate
/// references to other owned resources (brews, alert rules).
#[derive(Debug)]
pub enum WriteError {
    /// The resource itself does not exist (or belongs to someone else).
    NotFound,
    /// It references a brew, hydrometer or alert target the caller does not own.
    BadReference,
    /// The account is at its cap for this kind of resource (see `limits`).
    LimitReached(&'static str),
    #[allow(dead_code)]
    Db(DbErr),
}

impl From<DbErr> for WriteError {
    fn from(e: DbErr) -> Self {
        WriteError::Db(e)
    }
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WriteError::NotFound => write!(f, "not found"),
            WriteError::BadReference => write!(f, "references a resource the caller does not own"),
            WriteError::LimitReached(what) => write!(f, "account limit reached for {what}"),
            WriteError::Db(e) => write!(f, "database error: {e}"),
        }
    }
}
