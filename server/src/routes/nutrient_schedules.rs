use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Route, State, delete, get, post, routes};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use shared::{
    CreateNutrientSchedule, NutrientCalculateRequest, NutrientCalculateResponse,
    NutrientScheduleResponse,
};

use crate::guards::current_user::CurrentUser;
use crate::services::{brew_event_service, brew_service, nutrient_service};

#[post("/nutrients/calculate", data = "<input>")]
async fn calculate(
    _user: CurrentUser,
    input: Json<NutrientCalculateRequest>,
) -> Json<NutrientCalculateResponse> {
    Json(nutrient_service::calculate_nutrient_plan(
        &input.into_inner(),
    ))
}

#[get("/brews/<brew_id>/nutrient-schedule")]
async fn get_schedule(
    _user: CurrentUser,
    db: &State<DatabaseConnection>,
    brew_id: &str,
) -> Result<Json<NutrientScheduleResponse>, Status> {
    let brew_id = Uuid::parse_str(brew_id).map_err(|_| Status::UnprocessableEntity)?;
    match nutrient_service::get_schedule(db.inner(), brew_id).await {
        Ok(Some(s)) => Ok(Json(s)),
        Ok(None) => Err(Status::NotFound),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[post("/brews/<brew_id>/nutrient-schedule", data = "<input>")]
async fn create_schedule(
    _user: CurrentUser,
    db: &State<DatabaseConnection>,
    brew_id: &str,
    input: Json<CreateNutrientSchedule>,
) -> Result<(Status, Json<NutrientScheduleResponse>), Status> {
    let brew_id = Uuid::parse_str(brew_id).map_err(|_| Status::UnprocessableEntity)?;

    // Verify brew exists
    let brew = brew_service::find_by_id(db.inner(), brew_id)
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;

    // Check no existing schedule
    if let Ok(Some(_)) = nutrient_service::get_schedule(db.inner(), brew_id).await {
        return Err(Status::Conflict);
    }

    // Auto-create yeast_pitch event if none exists
    if let Some(start_date) = brew.start_date {
        let events = brew_event_service::find_by_brew(db.inner(), brew_id, None, None)
            .await
            .map_err(|_| Status::InternalServerError)?;

        let has_pitch = events
            .iter()
            .any(|e| e.event_type == shared::BrewEventType::YeastPitch);

        if !has_pitch {
            let pitch_event = shared::CreateBrewEvent {
                brew_id,
                event_type: shared::BrewEventType::YeastPitch,
                label: "Yeast pitched (auto)".to_string(),
                notes: Some("Auto-created when nutrient schedule was set up".to_string()),
                gravity_at_event: brew.og,
                temp_at_event: None,
                event_time: start_date,
            };
            let _ = brew_event_service::create(db.inner(), pitch_event).await;
        }
    }

    nutrient_service::create_schedule(db.inner(), brew_id, &input.into_inner())
        .await
        .map(|s| (Status::Created, Json(s)))
        .map_err(|_| Status::UnprocessableEntity)
}

#[delete("/brews/<brew_id>/nutrient-schedule")]
async fn delete_schedule(
    _user: CurrentUser,
    db: &State<DatabaseConnection>,
    brew_id: &str,
) -> Status {
    let Ok(brew_id) = Uuid::parse_str(brew_id) else {
        return Status::UnprocessableEntity;
    };
    match nutrient_service::delete_schedule(db.inner(), brew_id).await {
        Ok(true) => Status::NoContent,
        Ok(false) => Status::NotFound,
        Err(_) => Status::InternalServerError,
    }
}

pub fn routes() -> Vec<Route> {
    routes![calculate, get_schedule, create_schedule, delete_schedule]
}
