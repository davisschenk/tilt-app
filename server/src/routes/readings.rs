use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Route, State, get, post, routes};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use shared::{CreateReadingsBatch, ReadingResponse, ReadingsQuery, TiltReading};

use crate::guards::auth_or_api_key::AuthOrApiKey;
use crate::guards::current_user::CurrentUser;
use crate::services::{
    alert_rule_service, alert_target_service, brew_service, hydrometer_service, reading_service,
    tosna_service, webhook_dispatcher,
};

#[post("/readings", data = "<batch>")]
async fn create_batch(
    _auth: AuthOrApiKey,
    db: &State<DatabaseConnection>,
    http_client: &State<reqwest::Client>,
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

    for (color, batch_readings) in &grouped {
        let hydrometer = hydrometer_service::find_or_create_by_color(db.inner(), color)
            .await
            .map_err(|e| {
                tracing::error!(color = ?color, error = %e, "Failed to find/create hydrometer");
                Status::InternalServerError
            })?;

        let active_brew = brew_service::find_active_for_hydrometer(db.inner(), hydrometer.id)
            .await
            .unwrap_or(None);
        let brew_id = active_brew.as_ref().map(|b| b.id);

        let count = reading_service::batch_create(
            db.inner(),
            batch_readings.clone(),
            hydrometer.id,
            brew_id,
        )
        .await
        .map_err(|e| {
            tracing::error!(hydrometer_id = %hydrometer.id, error = %e, "Failed to batch create readings");
            Status::InternalServerError
        })?;

        total_count += count;

        // Alert + TOSNA evaluation: spawn as background task so it never blocks the response
        if let Some(latest) = batch_readings.last() {
            let db_ref = db.inner().clone();
            let client_ref = http_client.inner().clone();
            let gravity = latest.gravity;
            let temp_f = latest.temperature_f;
            let recorded_at = latest.recorded_at;
            let hydro_id = hydrometer.id;

            // Capture yeast strain for temperature safety check
            let temp_safety_ctx = active_brew.as_ref().and_then(|b| {
                let strain = b.yeast_strain.clone()?;
                Some((b.id, b.name.clone(), strain, b.nutrient_alert_target_id))
            });

            // Capture TOSNA fields if the brew has them configured
            let tosna_ctx = active_brew.as_ref().and_then(|b| {
                Some((
                    b.id,
                    b.name.clone(),
                    b.og?,
                    b.target_fg?,
                    b.batch_size_gallons?,
                    b.yeast_nitrogen_requirement
                        .clone()
                        .unwrap_or_else(|| "medium".to_string()),
                    b.nutrient_protocol
                        .clone()
                        .unwrap_or_else(|| "tosna_2".to_string()),
                    chrono::DateTime::<chrono::Utc>::from(b.pitch_time?),
                    b.nutrient_alert_target_id,
                ))
            });

            tokio::spawn(async move {
                evaluate_alerts(
                    &db_ref,
                    &client_ref,
                    gravity,
                    temp_f,
                    recorded_at,
                    brew_id,
                    Some(hydro_id),
                )
                .await;

                if let Some((
                    bid,
                    bname,
                    og,
                    target_fg,
                    gallons,
                    n_req,
                    protocol,
                    pitch_time,
                    alert_target_id,
                )) = tosna_ctx
                {
                    tosna_service::evaluate_due_additions(
                        &db_ref,
                        &client_ref,
                        bid,
                        &bname,
                        og,
                        target_fg,
                        gallons,
                        &n_req,
                        &protocol,
                        pitch_time,
                        gravity,
                        recorded_at,
                        alert_target_id,
                    )
                    .await;
                }

                if let Some((bid, bname, strain, alert_target_id)) = temp_safety_ctx {
                    tosna_service::evaluate_temperature_safety(
                        &db_ref,
                        &client_ref,
                        bid,
                        &bname,
                        &strain,
                        temp_f,
                        recorded_at,
                        alert_target_id,
                    )
                    .await;
                }
            });
        }
    }

    Ok((
        Status::Created,
        Json(serde_json::json!({ "count": total_count })),
    ))
}

/// Fire-and-forget alert evaluation. Errors are logged, never propagated.
async fn evaluate_alerts(
    db: &DatabaseConnection,
    http_client: &reqwest::Client,
    gravity: f64,
    temperature_f: f64,
    recorded_at: chrono::DateTime<chrono::Utc>,
    brew_id: Option<Uuid>,
    hydrometer_id: Option<Uuid>,
) {
    let triggered = match alert_rule_service::find_triggered_rules(
        db,
        gravity,
        temperature_f,
        brew_id,
        hydrometer_id,
    )
    .await
    {
        Ok(rules) => rules,
        Err(e) => {
            tracing::error!(error = %e, "Failed to query triggered alert rules");
            return;
        }
    };

    for rule in triggered {
        let target = match alert_target_service::find_by_id_raw(db, rule.alert_target_id).await {
            Ok(Some(t)) => t,
            Ok(None) => {
                tracing::warn!(alert_target_id = %rule.alert_target_id, "Alert target not found for triggered rule");
                continue;
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to look up alert target");
                continue;
            }
        };

        if !target.enabled {
            continue;
        }

        let rule_id = rule.id;
        match webhook_dispatcher::dispatch(
            http_client,
            &target,
            &rule,
            gravity,
            temperature_f,
            recorded_at,
        )
        .await
        {
            Ok(()) => {
                if let Err(e) = alert_rule_service::update_last_triggered(db, rule_id).await {
                    tracing::error!(rule_id = %rule_id, error = %e, "Failed to update last_triggered_at");
                }
            }
            Err(e) => {
                tracing::error!(rule_id = %rule_id, error = %e, "Webhook dispatch failed");
            }
        }
    }
}

#[get("/readings?<brew_id>&<hydrometer_id>&<since>&<until>&<limit>")]
async fn query(
    _user: CurrentUser,
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
