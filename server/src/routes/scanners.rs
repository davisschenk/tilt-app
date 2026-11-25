use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Route, State, get, post, routes};
use sea_orm::DatabaseConnection;
use serde_json::Value;

use crate::guards::{auth_or_api_key::AuthOrApiKey, current_user::CurrentUser};
use crate::services::WriteError;
use crate::services::scanner_service::{self, Heartbeat};

#[post("/scanners/heartbeat", data = "<input>")]
async fn heartbeat(
    auth: AuthOrApiKey,
    db: &State<DatabaseConnection>,
    input: Json<Heartbeat>,
) -> Result<Status, Status> {
    if !input.is_valid() {
        return Err(Status::UnprocessableEntity);
    }
    match scanner_service::record_heartbeat(db.inner(), auth.owner_id, &input).await {
        Ok(()) => Ok(Status::NoContent),
        Err(WriteError::LimitReached(_)) => Err(Status::Forbidden),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[get("/scanners")]
async fn list_scanners(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
) -> Result<Json<Vec<Value>>, Status> {
    scanner_service::list(db.inner(), user.user_id)
        .await
        .map(Json)
        .map_err(|_| Status::InternalServerError)
}

pub fn routes() -> Vec<Route> {
    routes![heartbeat, list_scanners]
}
