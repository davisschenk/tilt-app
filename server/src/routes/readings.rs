use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Route, State, get, post, routes};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use shared::{CreateReadingsBatch, ReadingResponse, ReadingsQuery, TiltReading};

use crate::services::{brew_service, hydrometer_service, reading_service};

#[post("/readings", data = "<batch>")]
async fn create_batch(
    db: &State<DatabaseConnection>,
    batch: Json<CreateReadingsBatch>,
) -> Result<(Status, Json<serde_json::Value>), Status> {
    let readings: Vec<TiltReading> = batch.into_inner().0;
    if readings.is_empty() {
        return Ok((Status::Created, Json(serde_json::json!({ "count": 0 }))));
    }

    let mut total_count: u64 = 0;

    let mut grouped: std::collections::HashMap<shared::TiltColor, Vec<TiltReading>> =
        std::collections::HashMap::new();
    for r in readings {
        grouped.entry(r.color).or_default().push(r);
    }

    for (color, batch_readings) in grouped {
        let hydrometer = hydrometer_service::find_or_create_by_color(db.inner(), &color)
            .await
            .map_err(|e| {
                tracing::error!(color = ?color, error = %e, "Failed to find/create hydrometer");
                Status::InternalServerError
            })?;

        let active_brew = brew_service::find_active_for_hydrometer(db.inner(), hydrometer.id).await;
        let brew_id = match active_brew {
            Ok(Some(b)) => Some(b.id),
            Ok(None) => None,
            Err(_) => None,
        };

        let count =
            reading_service::batch_create(db.inner(), batch_readings, hydrometer.id, brew_id)
                .await
                .map_err(|e| {
                    tracing::error!(hydrometer_id = %hydrometer.id, error = %e, "Failed to batch create readings");
                    Status::InternalServerError
                })?;

        total_count += count;
    }

    Ok((
        Status::Created,
        Json(serde_json::json!({ "count": total_count })),
    ))
}

#[get("/readings?<brew_id>&<hydrometer_id>&<since>&<until>&<limit>")]
async fn query(
    db: &State<DatabaseConnection>,
    brew_id: Option<&str>,
    hydrometer_id: Option<&str>,
    since: Option<&str>,
    until: Option<&str>,
    limit: Option<u64>,
) -> Result<Json<Vec<ReadingResponse>>, Status> {
    let query = ReadingsQuery {
        brew_id: brew_id.and_then(|s| Uuid::parse_str(s).ok()),
        hydrometer_id: hydrometer_id.and_then(|s| Uuid::parse_str(s).ok()),
        since: since.and_then(|s| s.parse().ok()),
        until: until.and_then(|s| s.parse().ok()),
        limit,
    };

    reading_service::find_filtered(db.inner(), &query)
        .await
        .map(Json)
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to query readings");
            Status::InternalServerError
        })
}

pub fn routes() -> Vec<Route> {
    routes![create_batch, query]
}
