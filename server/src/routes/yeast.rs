use rocket::serde::json::Json;
use rocket::{Route, State, get, routes};
use sea_orm::DatabaseConnection;

use crate::guards::current_user::CurrentUser;
use crate::services::tosna_service::{YEAST_STRAIN_TABLE, YeastStrainInfo};

#[get("/yeast-strains")]
async fn list_strains(
    _user: CurrentUser,
    _db: &State<DatabaseConnection>,
) -> Json<&'static [YeastStrainInfo]> {
    Json(YEAST_STRAIN_TABLE)
}

pub fn routes() -> Vec<Route> {
    routes![list_strains]
}
