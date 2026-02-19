use rocket::{
    Request,
    http::Status,
    request::{FromRequest, Outcome},
};
use sea_orm::DatabaseConnection;

use crate::services::{
    api_keys::{validate_api_key, ApiKeyError},
    sessions,
};
use crate::guards::current_user::SESSION_COOKIE;
use uuid::Uuid;

/// A guard that accepts either a valid session cookie (CurrentUser) OR a valid API key.
/// Used on the POST /readings endpoint so both the web UI and the Pi client can submit readings.
pub struct AuthOrApiKey;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthOrApiKey {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let db = match req.guard::<&rocket::State<DatabaseConnection>>().await {
            Outcome::Success(db) => db,
            _ => return Outcome::Error((Status::InternalServerError, ())),
        };

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
            match validate_api_key(db, &raw_key).await {
                Ok(_) => return Outcome::Success(AuthOrApiKey),
                Err(ApiKeyError::Expired) => return Outcome::Error((Status::Unauthorized, ())),
                Err(ApiKeyError::Invalid) => {} // fall through to session check
                Err(ApiKeyError::Db(_)) => return Outcome::Error((Status::InternalServerError, ())),
            }
        }

        // Try session cookie
        let session_id_str = match req.cookies().get_private(SESSION_COOKIE) {
            Some(c) => c.value().to_string(),
            None => return Outcome::Error((Status::Unauthorized, ())),
        };

        let session_id = match Uuid::parse_str(&session_id_str) {
            Ok(id) => id,
            Err(_) => return Outcome::Error((Status::Unauthorized, ())),
        };

        match sessions::get_session_by_id(db, session_id).await {
            Ok(Some(session)) => {
                let now = chrono::Utc::now().fixed_offset();
                if session.expires_at < now {
                    return Outcome::Error((Status::Unauthorized, ()));
                }
                let _ = sessions::touch_session(db, session_id).await;
                Outcome::Success(AuthOrApiKey)
            }
            Ok(None) => Outcome::Error((Status::Unauthorized, ())),
            Err(_) => Outcome::Error((Status::InternalServerError, ())),
        }
    }
}
