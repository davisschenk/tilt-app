use std::collections::HashMap;

use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Route, State, get, routes};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use shared::{NutrientProduct, NutrientProtocol, NutrientScheduleResponse};

use crate::guards::current_user::CurrentUser;
use crate::services::{brew_service, tosna_service};

fn product_key(p: NutrientProduct) -> String {
    match p {
        NutrientProduct::FermaidO => "fermaid_o".to_string(),
        NutrientProduct::FermaidK => "fermaid_k".to_string(),
        NutrientProduct::Dap => "dap".to_string(),
        NutrientProduct::GoFerm => "go_ferm".to_string(),
    }
}

#[get("/brews/<brew_id>/nutrient-schedule")]
async fn get_nutrient_schedule(
    _user: CurrentUser,
    db: &State<DatabaseConnection>,
    brew_id: &str,
) -> Result<Json<NutrientScheduleResponse>, (Status, Json<serde_json::Value>)> {
    let id = Uuid::parse_str(brew_id).map_err(|_| {
        (
            Status::UnprocessableEntity,
            Json(serde_json::json!({"error": "invalid brew_id UUID"})),
        )
    })?;

    let brew = brew_service::find_by_id(db.inner(), id)
        .await
        .map_err(|_| {
            (
                Status::InternalServerError,
                Json(serde_json::json!({"error": "database error"})),
            )
        })?
        .ok_or_else(|| {
            (
                Status::NotFound,
                Json(serde_json::json!({"error": "brew not found"})),
            )
        })?;

    let og = brew.og.ok_or_else(|| {
        (
            Status::UnprocessableEntity,
            Json(serde_json::json!({"error": "brew is missing og (original gravity)"})),
        )
    })?;

    let target_fg = brew.target_fg.ok_or_else(|| {
        (
            Status::UnprocessableEntity,
            Json(serde_json::json!({"error": "brew is missing target_fg"})),
        )
    })?;

    let batch_size_gallons = brew.batch_size_gallons.ok_or_else(|| {
        (
            Status::UnprocessableEntity,
            Json(serde_json::json!({"error": "brew is missing batch_size_gallons"})),
        )
    })?;

    let pitch_time = brew.pitch_time.ok_or_else(|| {
        (
            Status::UnprocessableEntity,
            Json(serde_json::json!({"error": "brew is missing pitch_time"})),
        )
    })?;

    let (nitrogen_req, resolved_from_strain) =
        if let Some(ref req) = brew.yeast_nitrogen_requirement {
            (req.clone(), false)
        } else if let Some(strain) = brew
            .yeast_strain
            .as_deref()
            .and_then(tosna_service::lookup_strain)
        {
            (strain.nitrogen_requirement.to_string(), true)
        } else {
            ("medium".to_string(), false)
        };

    let protocol_str = brew.nutrient_protocol.as_deref().unwrap_or("tosna_2");
    let protocol = NutrientProtocol::from_protocol_str(protocol_str);

    let additions = tosna_service::compute_schedule(
        protocol,
        og,
        target_fg,
        batch_size_gallons,
        &nitrogen_req,
        pitch_time,
    );

    let total_yan = tosna_service::required_yan_ppm(og, &nitrogen_req);
    let batch_size_liters = tosna_service::gallons_to_liters(batch_size_gallons);

    let mut nutrient_totals: HashMap<String, f64> = HashMap::new();
    for addition in &additions {
        *nutrient_totals
            .entry(product_key(addition.product))
            .or_insert(0.0) += addition.amount_grams;
    }

    Ok(Json(NutrientScheduleResponse {
        protocol: protocol_str.to_string(),
        additions,
        total_yan_required_ppm: total_yan,
        nutrient_totals,
        batch_size_gallons,
        batch_size_liters,
        og,
        target_fg,
        nitrogen_requirement: nitrogen_req,
        pitch_time,
        resolved_from_strain,
    }))
}

pub fn routes() -> Vec<Route> {
    routes![get_nutrient_schedule]
}
