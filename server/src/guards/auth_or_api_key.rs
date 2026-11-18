use rocket::{
    Request,
    http::Status,
    request::{FromRequest, Outcome},
};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::guards::current_user::CurrentUser;
use crate::services::api_keys::{ApiKeyError, validate_api_key};

/// Accepts either a valid API key or a valid session, and says whose data the
/// request acts on. Used on `POST /readings` so both the web UI and the ESP32
/// can submit readings. `owner_id` MUST scope every query the handler runs.
pub struct AuthOrApiKey {
    pub owner_id: Uuid,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthOrApiKey {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Try API key first (X-API-Key or Authorization: Bearer)
        let raw_key = req
            .headers()
            .get_one("X-API-Key")
            .or_else(|| {
                req.headers()
                    .get_one("Authorization")
                    .and_then(|v| v.strip_prefix("Bearer "))
            })
            .map(|s| s.to_string());

        if let Some(raw_key) = raw_key {
            let db = match req.guard::<&rocket::State<DatabaseConnection>>().await {
                Outcome::Success(db) => db,
                _ => return Outcome::Error((Status::InternalServerError, ())),
            };
            match validate_api_key(db, &raw_key).await {
                Ok(key) => {
                    return Outcome::Success(AuthOrApiKey {
                        owner_id: key.owner_id,
                    });
                }
                Err(ApiKeyError::Expired) => return Outcome::Error((Status::Unauthorized, ())),
                Err(ApiKeyError::Invalid) => {} // fall through to session check
                Err(ApiKeyError::Db(_)) => {
                    return Outcome::Error((Status::InternalServerError, ()));
                }
            }
        }

        // Otherwise fall back to the session (or the dev user when auth is disabled).
        match req.guard::<CurrentUser>().await {
            Outcome::Success(user) => Outcome::Success(AuthOrApiKey {
                owner_id: user.user_id,
            }),
            Outcome::Error(e) => Outcome::Error(e),
            Outcome::Forward(f) => Outcome::Forward(f),
        }
    }
}
