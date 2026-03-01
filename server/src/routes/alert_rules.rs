use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Route, State, delete, get, post, put, routes};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use shared::{AlertRuleResponse, CreateAlertRule, UpdateAlertRule};

use crate::guards::current_user::CurrentUser;
use crate::services::alert_rule_service;

#[get("/alert-rules?<brew_id>&<hydrometer_id>")]
async fn list(
    _user: CurrentUser,
    db: &State<DatabaseConnection>,
    brew_id: Option<&str>,
    hydrometer_id: Option<&str>,
) -> Result<Json<Vec<AlertRuleResponse>>, Status> {
    let brew_id = brew_id
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|_| Status::UnprocessableEntity)?;
    let hydrometer_id = hydrometer_id
        .map(Uuid::parse_str)
        .transpose()
        .map_err(|_| Status::UnprocessableEntity)?;

    alert_rule_service::find_all(db.inner(), brew_id, hydrometer_id)
        .await
        .map(Json)
        .map_err(|_| Status::InternalServerError)
}

#[get("/alert-rules/<id>")]
async fn get_by_id(
    _user: CurrentUser,
    db: &State<DatabaseConnection>,
    id: &str,
) -> Result<Json<AlertRuleResponse>, Status> {
    let id = Uuid::parse_str(id).map_err(|_| Status::UnprocessableEntity)?;
    match alert_rule_service::find_by_id(db.inner(), id).await {
        Ok(Some(r)) => Ok(Json(r)),
        Ok(None) => Err(Status::NotFound),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[post("/alert-rules", data = "<input>")]
async fn create(
    _user: CurrentUser,
    db: &State<DatabaseConnection>,
    input: Json<CreateAlertRule>,
) -> Result<(Status, Json<AlertRuleResponse>), Status> {
    alert_rule_service::create(db.inner(), input.into_inner())
        .await
        .map(|r| (Status::Created, Json(r)))
        .map_err(|_| Status::UnprocessableEntity)
}

#[put("/alert-rules/<id>", data = "<input>")]
async fn update(
    _user: CurrentUser,
    db: &State<DatabaseConnection>,
    id: &str,
    input: Json<UpdateAlertRule>,
) -> Result<Json<AlertRuleResponse>, Status> {
    let id = Uuid::parse_str(id).map_err(|_| Status::UnprocessableEntity)?;
    match alert_rule_service::update(db.inner(), id, input.into_inner()).await {
        Ok(Some(r)) => Ok(Json(r)),
        Ok(None) => Err(Status::NotFound),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[delete("/alert-rules/<id>")]
async fn delete_rule(_user: CurrentUser, db: &State<DatabaseConnection>, id: &str) -> Status {
    let Ok(id) = Uuid::parse_str(id) else {
        return Status::UnprocessableEntity;
    };
    match alert_rule_service::delete(db.inner(), id).await {
        Ok(true) => Status::NoContent,
        Ok(false) => Status::NotFound,
        Err(_) => Status::InternalServerError,
    }
}

pub fn routes() -> Vec<Route> {
    routes![list, get_by_id, create, update, delete_rule]
}
