use chrono::DateTime;
use rocket::{
    State, delete, get, post,
    http::Status,
    serde::json::Json,
};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    guards::current_user::CurrentUser,
    services::api_keys::{self, ApiKeyError},
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub expires_at: Option<DateTime<chrono::FixedOffset>>,
}

#[get("/api-keys")]
pub async fn list(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
) -> Result<Json<serde_json::Value>, Status> {
    api_keys::list_api_keys(db.inner(), &user.user_sub)
        .await
        .map(|keys| Json(serde_json::json!(keys)))
        .map_err(|_| Status::InternalServerError)
}

#[post("/api-keys", data = "<input>")]
pub async fn create(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    input: Json<CreateApiKeyRequest>,
) -> Result<(Status, Json<serde_json::Value>), Status> {
    let req = input.into_inner();
    api_keys::create_api_key(db.inner(), req.name, user.user_sub, req.expires_at)
        .await
        .map(|created| (Status::Created, Json(serde_json::json!(created))))
        .map_err(|_| Status::InternalServerError)
}

#[delete("/api-keys/<id>")]
pub async fn delete(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    id: &str,
) -> Status {
    let Ok(id) = Uuid::parse_str(id) else {
        return Status::UnprocessableEntity;
    };
    match api_keys::delete_api_key(db.inner(), id, &user.user_sub).await {
        Ok(()) => Status::NoContent,
        Err(ApiKeyError::Invalid) => Status::Forbidden,
        Err(_) => Status::InternalServerError,
    }
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![list, create, delete]
}
