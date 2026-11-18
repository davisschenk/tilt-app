use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Route, State, delete, get, post, put, routes};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use shared::{AlertTargetResponse, CreateAlertTarget, UpdateAlertTarget};

use crate::fairings::rate_limit::TestFireLimiter;
use crate::guards::current_user::CurrentUser;
use crate::services::{alert_target_service, webhook_dispatcher, webhook_guard};

#[get("/alert-targets")]
async fn list(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
) -> Result<Json<Vec<AlertTargetResponse>>, Status> {
    alert_target_service::find_all(db.inner(), user.user_id)
        .await
        .map(Json)
        .map_err(|_| Status::InternalServerError)
}

#[get("/alert-targets/<id>")]
async fn get_by_id(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    id: &str,
) -> Result<Json<AlertTargetResponse>, Status> {
    let id = Uuid::parse_str(id).map_err(|_| Status::UnprocessableEntity)?;
    match alert_target_service::find_by_id(db.inner(), user.user_id, id).await {
        Ok(Some(t)) => Ok(Json(t)),
        Ok(None) => Err(Status::NotFound),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[post("/alert-targets", data = "<input>")]
async fn create(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    input: Json<CreateAlertTarget>,
) -> Result<(Status, Json<AlertTargetResponse>), Status> {
    let input = input.into_inner();
    if webhook_guard::validate_url(&input.url).is_err() {
        return Err(Status::UnprocessableEntity);
    }
    alert_target_service::create(db.inner(), user.user_id, input)
        .await
        .map(|t| (Status::Created, Json(t)))
        .map_err(|_| Status::UnprocessableEntity)
}

#[put("/alert-targets/<id>", data = "<input>")]
async fn update(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    id: &str,
    input: Json<UpdateAlertTarget>,
) -> Result<Json<AlertTargetResponse>, Status> {
    let id = Uuid::parse_str(id).map_err(|_| Status::UnprocessableEntity)?;
    let input = input.into_inner();
    if let Some(url) = &input.url
        && webhook_guard::validate_url(url).is_err()
    {
        return Err(Status::UnprocessableEntity);
    }
    match alert_target_service::update(db.inner(), user.user_id, id, input).await {
        Ok(Some(t)) => Ok(Json(t)),
        Ok(None) => Err(Status::NotFound),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[delete("/alert-targets/<id>")]
async fn delete_target(user: CurrentUser, db: &State<DatabaseConnection>, id: &str) -> Status {
    let Ok(id) = Uuid::parse_str(id) else {
        return Status::UnprocessableEntity;
    };
    match alert_target_service::delete(db.inner(), user.user_id, id).await {
        Ok(true) => Status::NoContent,
        Ok(false) => Status::NotFound,
        Err(_) => Status::InternalServerError,
    }
}

#[post("/alert-targets/<id>/test")]
async fn test_fire(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    http_client: &State<reqwest::Client>,
    limiter: &State<TestFireLimiter>,
    id: &str,
) -> Result<Json<serde_json::Value>, Status> {
    if !limiter.allow(user.user_id) {
        return Err(Status::TooManyRequests);
    }
    let Ok(id) = Uuid::parse_str(id) else {
        return Ok(Json(
            serde_json::json!({ "ok": false, "error": "Invalid UUID" }),
        ));
    };

    let target = match alert_target_service::find_raw_by_id(db.inner(), user.user_id, id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return Ok(Json(
                serde_json::json!({ "ok": false, "error": "Alert target not found" }),
            ));
        }
        Err(e) => {
            return Ok(Json(
                serde_json::json!({ "ok": false, "error": format!("Database error: {e}") }),
            ));
        }
    };

    Ok(Json(
        match webhook_dispatcher::dispatch_test(http_client.inner(), &target).await {
            Ok(status_code) => serde_json::json!({ "ok": true, "statusCode": status_code }),
            Err(e) => serde_json::json!({ "ok": false, "error": format!("{e}") }),
        },
    ))
}

pub fn routes() -> Vec<Route> {
    routes![list, get_by_id, create, update, delete_target, test_fire]
}
