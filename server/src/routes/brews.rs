use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Route, State, delete, get, post, put, routes};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use shared::{BrewResponse, CreateBrew, UpdateBrew};

use crate::guards::current_user::CurrentUser;
use crate::services::brew_service::{self, WriteError};

#[get("/brews?<status>")]
async fn list(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    status: Option<&str>,
) -> Result<Json<Vec<BrewResponse>>, Status> {
    brew_service::find_all(db.inner(), user.user_id, status)
        .await
        .map(Json)
        .map_err(|_| Status::InternalServerError)
}

#[get("/brews/<id>")]
async fn get_by_id(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    id: &str,
) -> Result<Json<BrewResponse>, Status> {
    let id = Uuid::parse_str(id).map_err(|_| Status::UnprocessableEntity)?;
    match brew_service::find_by_id(db.inner(), user.user_id, id).await {
        Ok(Some(b)) => Ok(Json(b)),
        Ok(None) => Err(Status::NotFound),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[post("/brews", data = "<input>")]
async fn create(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    input: Json<CreateBrew>,
) -> Result<(Status, Json<BrewResponse>), Status> {
    match brew_service::create(db.inner(), user.user_id, input.into_inner()).await {
        Ok(b) => Ok((Status::Created, Json(b))),
        Err(WriteError::BadReference | WriteError::NotFound) => Err(Status::UnprocessableEntity),
        Err(WriteError::Db(_)) => Err(Status::InternalServerError),
    }
}

#[put("/brews/<id>", data = "<input>")]
async fn update(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    id: &str,
    input: Json<UpdateBrew>,
) -> Result<Json<BrewResponse>, Status> {
    let id = Uuid::parse_str(id).map_err(|_| Status::UnprocessableEntity)?;
    match brew_service::update(db.inner(), user.user_id, id, input.into_inner()).await {
        Ok(b) => Ok(Json(b)),
        Err(WriteError::NotFound) => Err(Status::NotFound),
        Err(WriteError::BadReference) => Err(Status::UnprocessableEntity),
        Err(WriteError::Db(_)) => Err(Status::InternalServerError),
    }
}

#[delete("/brews/<id>")]
async fn delete(user: CurrentUser, db: &State<DatabaseConnection>, id: &str) -> Status {
    let Ok(id) = Uuid::parse_str(id) else {
        return Status::UnprocessableEntity;
    };
    match brew_service::delete(db.inner(), user.user_id, id).await {
        Ok(true) => Status::NoContent,
        Ok(false) => Status::NotFound,
        Err(_) => Status::InternalServerError,
    }
}

pub fn routes() -> Vec<Route> {
    routes![list, get_by_id, create, update, delete]
}
