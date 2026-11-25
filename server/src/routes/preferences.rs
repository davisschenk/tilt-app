use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Route, State, get, put, routes};
use sea_orm::DatabaseConnection;

use crate::guards::current_user::CurrentUser;
use crate::services::preferences_service::{self, Preferences};

#[get("/account/preferences")]
async fn get_preferences(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
) -> Result<Json<Preferences>, Status> {
    preferences_service::get(db.inner(), user.user_id)
        .await
        .map(Json)
        .map_err(|_| Status::InternalServerError)
}

#[put("/account/preferences", data = "<input>")]
async fn set_preferences(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    input: Json<Preferences>,
) -> Result<Json<Preferences>, Status> {
    preferences_service::set(db.inner(), user.user_id, &input)
        .await
        .map_err(|_| Status::InternalServerError)?;
    Ok(input)
}

pub fn routes() -> Vec<Route> {
    routes![get_preferences, set_preferences]
}
