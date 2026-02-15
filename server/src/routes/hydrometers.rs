use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Route, State, delete, get, post, put, routes};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use shared::{CreateHydrometer, HydrometerResponse, UpdateHydrometer};

use crate::services::hydrometer_service;

#[get("/hydrometers")]
async fn list(db: &State<DatabaseConnection>) -> Result<Json<Vec<HydrometerResponse>>, Status> {
    hydrometer_service::find_all(db.inner())
        .await
        .map(Json)
        .map_err(|_| Status::InternalServerError)
}

#[get("/hydrometers/<id>")]
async fn get_by_id(
    db: &State<DatabaseConnection>,
    id: &str,
) -> Result<Json<HydrometerResponse>, Status> {
    let id = Uuid::parse_str(id).map_err(|_| Status::UnprocessableEntity)?;
    match hydrometer_service::find_by_id(db.inner(), id).await {
        Ok(Some(h)) => Ok(Json(h)),
        Ok(None) => Err(Status::NotFound),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[post("/hydrometers", data = "<input>")]
async fn create(
    db: &State<DatabaseConnection>,
    input: Json<CreateHydrometer>,
) -> Result<(Status, Json<HydrometerResponse>), Status> {
    hydrometer_service::create(db.inner(), input.into_inner())
        .await
        .map(|h| (Status::Created, Json(h)))
        .map_err(|_| Status::UnprocessableEntity)
}

#[put("/hydrometers/<id>", data = "<input>")]
async fn update(
    db: &State<DatabaseConnection>,
    id: &str,
    input: Json<UpdateHydrometer>,
) -> Result<Json<HydrometerResponse>, Status> {
    let id = Uuid::parse_str(id).map_err(|_| Status::UnprocessableEntity)?;
    match hydrometer_service::update(db.inner(), id, input.into_inner()).await {
        Ok(Some(h)) => Ok(Json(h)),
        Ok(None) => Err(Status::NotFound),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[delete("/hydrometers/<id>")]
async fn delete(db: &State<DatabaseConnection>, id: &str) -> Status {
    let Ok(id) = Uuid::parse_str(id) else {
        return Status::UnprocessableEntity;
    };
    match hydrometer_service::delete(db.inner(), id).await {
        Ok(true) => Status::NoContent,
        Ok(false) => Status::NotFound,
        Err(_) => Status::InternalServerError,
    }
}

pub fn routes() -> Vec<Route> {
    routes![list, get_by_id, create, update, delete]
}
