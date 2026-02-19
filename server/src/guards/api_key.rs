use rocket::{
    Request,
    http::Status,
    request::{FromRequest, Outcome},
};
use sea_orm::DatabaseConnection;

use crate::models::entities::api_keys;
use crate::services::api_keys::{validate_api_key, ApiKeyError};

pub struct ApiKeyGuard {
    pub key: api_keys::Model,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ApiKeyGuard {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let raw_key = req
            .headers()
            .get_one("X-API-Key")
            .or_else(|| {
                req.headers()
                    .get_one("Authorization")
                    .and_then(|v| v.strip_prefix("Bearer "))
            })
            .map(|s| s.to_string());

        let raw_key = match raw_key {
            Some(k) => k,
            None => return Outcome::Error((Status::Unauthorized, ())),
        };

        let db = match req.guard::<&rocket::State<DatabaseConnection>>().await {
            Outcome::Success(db) => db,
            _ => return Outcome::Error((Status::InternalServerError, ())),
        };

        match validate_api_key(db, &raw_key).await {
            Ok(key) => Outcome::Success(ApiKeyGuard { key }),
            Err(ApiKeyError::Expired) => Outcome::Error((Status::Unauthorized, ())),
            Err(ApiKeyError::Invalid) => Outcome::Error((Status::Unauthorized, ())),
            Err(ApiKeyError::Db(_)) => Outcome::Error((Status::InternalServerError, ())),
        }
    }
}
