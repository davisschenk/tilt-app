use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Route, State, delete, get, post, put, routes};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use shared::{AlertTargetResponse, CreateAlertTarget, UpdateAlertTarget};

use crate::guards::current_user::CurrentUser;
use crate::services::alert_target_service;

#[get("/alert-targets")]
async fn list(
    _user: CurrentUser,
    db: &State<DatabaseConnection>,
) -> Result<Json<Vec<AlertTargetResponse>>, Status> {
    alert_target_service::find_all(db.inner())
        .await
        .map(Json)
        .map_err(|_| Status::InternalServerError)
}

#[get("/alert-targets/<id>")]
async fn get_by_id(
    _user: CurrentUser,
    db: &State<DatabaseConnection>,
    id: &str,
) -> Result<Json<AlertTargetResponse>, Status> {
    let id = Uuid::parse_str(id).map_err(|_| Status::UnprocessableEntity)?;
    match alert_target_service::find_by_id(db.inner(), id).await {
        Ok(Some(t)) => Ok(Json(t)),
        Ok(None) => Err(Status::NotFound),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[post("/alert-targets", data = "<input>")]
async fn create(
    _user: CurrentUser,
    db: &State<DatabaseConnection>,
    input: Json<CreateAlertTarget>,
) -> Result<(Status, Json<AlertTargetResponse>), Status> {
    alert_target_service::create(db.inner(), input.into_inner())
        .await
        .map(|t| (Status::Created, Json(t)))
        .map_err(|_| Status::UnprocessableEntity)
}

#[put("/alert-targets/<id>", data = "<input>")]
async fn update(
    _user: CurrentUser,
    db: &State<DatabaseConnection>,
    id: &str,
    input: Json<UpdateAlertTarget>,
) -> Result<Json<AlertTargetResponse>, Status> {
    let id = Uuid::parse_str(id).map_err(|_| Status::UnprocessableEntity)?;
    match alert_target_service::update(db.inner(), id, input.into_inner()).await {
        Ok(Some(t)) => Ok(Json(t)),
        Ok(None) => Err(Status::NotFound),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[delete("/alert-targets/<id>")]
async fn delete_target(_user: CurrentUser, db: &State<DatabaseConnection>, id: &str) -> Status {
    let Ok(id) = Uuid::parse_str(id) else {
        return Status::UnprocessableEntity;
    };
    match alert_target_service::delete(db.inner(), id).await {
        Ok(true) => Status::NoContent,
        Ok(false) => Status::NotFound,
        Err(_) => Status::InternalServerError,
    }
}

pub fn routes() -> Vec<Route> {
    routes![list, get_by_id, create, update, delete_target]
}
