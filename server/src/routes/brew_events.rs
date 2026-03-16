use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Route, State, delete, get, post, put, routes};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use shared::{BrewEventResponse, CreateBrewEvent, UpdateBrewEvent};

use crate::guards::current_user::CurrentUser;
use crate::services::brew_event_service;

#[get("/brews/<brew_id>/events?<since>&<until>")]
async fn list(
    _user: CurrentUser,
    db: &State<DatabaseConnection>,
    brew_id: &str,
    since: Option<&str>,
    until: Option<&str>,
) -> Result<Json<Vec<BrewEventResponse>>, Status> {
    let brew_id = Uuid::parse_str(brew_id).map_err(|_| Status::UnprocessableEntity)?;
    let since = since
        .map(|s| s.parse::<chrono::DateTime<chrono::Utc>>())
        .transpose()
        .map_err(|_| Status::UnprocessableEntity)?;
    let until = until
        .map(|s| s.parse::<chrono::DateTime<chrono::Utc>>())
        .transpose()
        .map_err(|_| Status::UnprocessableEntity)?;

    brew_event_service::find_by_brew(db.inner(), brew_id, since, until)
        .await
        .map(Json)
        .map_err(|_| Status::InternalServerError)
}

#[get("/brews/<brew_id>/events/<id>")]
async fn get_by_id(
    _user: CurrentUser,
    db: &State<DatabaseConnection>,
    brew_id: &str,
    id: &str,
) -> Result<Json<BrewEventResponse>, Status> {
    let _brew_id = Uuid::parse_str(brew_id).map_err(|_| Status::UnprocessableEntity)?;
    let id = Uuid::parse_str(id).map_err(|_| Status::UnprocessableEntity)?;
    match brew_event_service::find_by_id(db.inner(), id).await {
        Ok(Some(r)) => Ok(Json(r)),
        Ok(None) => Err(Status::NotFound),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[post("/brews/<brew_id>/events", data = "<input>")]
async fn create(
    _user: CurrentUser,
    db: &State<DatabaseConnection>,
    brew_id: &str,
    input: Json<CreateBrewEvent>,
) -> Result<(Status, Json<BrewEventResponse>), Status> {
    let brew_id = Uuid::parse_str(brew_id).map_err(|_| Status::UnprocessableEntity)?;
    let mut payload = input.into_inner();
    payload.brew_id = brew_id;

    brew_event_service::create(db.inner(), payload)
        .await
        .map(|r| (Status::Created, Json(r)))
        .map_err(|_| Status::UnprocessableEntity)
}

#[put("/brews/<brew_id>/events/<id>", data = "<input>")]
async fn update(
    _user: CurrentUser,
    db: &State<DatabaseConnection>,
    brew_id: &str,
    id: &str,
    input: Json<UpdateBrewEvent>,
) -> Result<Json<BrewEventResponse>, Status> {
    let _brew_id = Uuid::parse_str(brew_id).map_err(|_| Status::UnprocessableEntity)?;
    let id = Uuid::parse_str(id).map_err(|_| Status::UnprocessableEntity)?;
    match brew_event_service::update(db.inner(), id, input.into_inner()).await {
        Ok(Some(r)) => Ok(Json(r)),
        Ok(None) => Err(Status::NotFound),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[delete("/brews/<brew_id>/events/<id>")]
async fn delete_event(
    _user: CurrentUser,
    db: &State<DatabaseConnection>,
    brew_id: &str,
    id: &str,
) -> Status {
    let Ok(_brew_id) = Uuid::parse_str(brew_id) else {
        return Status::UnprocessableEntity;
    };
    let Ok(id) = Uuid::parse_str(id) else {
        return Status::UnprocessableEntity;
    };
    match brew_event_service::delete(db.inner(), id).await {
        Ok(true) => Status::NoContent,
        Ok(false) => Status::NotFound,
        Err(_) => Status::InternalServerError,
    }
}

pub fn routes() -> Vec<Route> {
    routes![list, get_by_id, create, update, delete_event]
}
