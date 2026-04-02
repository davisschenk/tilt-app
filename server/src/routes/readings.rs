use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Route, State, get, post, routes};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use shared::{CreateReadingsBatch, ReadingResponse, ReadingsQuery, TiltReading};

use crate::guards::auth_or_api_key::AuthOrApiKey;
use crate::guards::current_user::CurrentUser;
use crate::services::{
    alert_rule_service, alert_target_service, brew_event_service, brew_service, hydrometer_service,
    nutrient_service, reading_service, webhook_dispatcher,
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

        let active_brew = brew_service::find_active_for_hydrometer(db.inner(), hydrometer.id).await;
        let brew_id = match active_brew {
            Ok(Some(b)) => Some(b.id),
            Ok(None) => None,
            Err(_) => None,
        };

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

        // Alert evaluation: spawn as background task so it never blocks the response
        if let Some(latest) = batch_readings.last() {
            let db_ref = db.inner().clone();
            let client_ref = http_client.inner().clone();
            let gravity = latest.gravity;
            let temp_f = latest.temperature_f;
            let recorded_at = latest.recorded_at;
            let hydro_id = hydrometer.id;
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

    // Nutrient schedule notifications
    if let Some(brew_id) = brew_id {
        evaluate_nutrient_additions(db, http_client, brew_id, gravity).await;
    }
}

async fn evaluate_nutrient_additions(
    db: &DatabaseConnection,
    http_client: &reqwest::Client,
    brew_id: Uuid,
    current_gravity: f64,
) {
    let schedule_data = match nutrient_service::find_pending_additions_for_brew(db, brew_id).await {
        Ok(Some(data)) => data,
        Ok(None) => return,
        Err(e) => {
            tracing::error!(brew_id = %brew_id, error = %e, "Failed to query nutrient schedule");
            return;
        }
    };

    let (schedule, pending_additions) = schedule_data;
    let alert_target_id = match schedule.alert_target_id {
        Some(id) => id,
        None => return,
    };

    // Find pitch time from brew events
    let pitch_time = match brew_event_service::find_by_brew(db, brew_id, None, None).await {
        Ok(events) => events
            .iter()
            .find(|e| e.event_type == shared::BrewEventType::YeastPitch)
            .map(|e| e.event_time),
        Err(_) => return,
    };

    let Some(pitch_time) = pitch_time else {
        return;
    };

    // Get brew name
    let brew_name = match brew_service::find_by_id(db, brew_id).await {
        Ok(Some(b)) => b.name,
        _ => "Unknown Brew".to_string(),
    };

    // Check only the lowest un-notified addition (sequential)
    let Some(addition) = pending_additions.first() else {
        return;
    };

    let now = chrono::Utc::now();
    let hours_since_pitch = (now - pitch_time).num_seconds() as f64 / 3600.0;

    let should_notify = match addition.trigger_type.as_str() {
        "time" => addition
            .target_hours
            .is_some_and(|h| hours_since_pitch >= h),
        "gravity_or_time" => {
            let time_triggered = addition
                .target_hours
                .is_some_and(|h| hours_since_pitch >= h);
            let gravity_triggered = addition
                .target_gravity
                .is_some_and(|g| current_gravity <= g);
            time_triggered || gravity_triggered
        }
        _ => false,
    };

    if !should_notify {
        return;
    }

    // Look up the alert target
    let target = match alert_target_service::find_by_id_raw(db, alert_target_id).await {
        Ok(Some(t)) if t.enabled => t,
        _ => return,
    };

    match webhook_dispatcher::dispatch_nutrient(
        http_client,
        &target,
        &brew_name,
        brew_id,
        addition,
        NUM_NUTRIENT_ADDITIONS,
        schedule.one_third_break_sg,
    )
    .await
    {
        Ok(()) => {
            if let Err(e) = nutrient_service::mark_addition_notified(db, addition.id).await {
                tracing::error!(addition_id = %addition.id, error = %e, "Failed to mark nutrient addition as notified");
            }
        }
        Err(e) => {
            tracing::error!(addition_id = %addition.id, error = %e, "Nutrient webhook dispatch failed");
        }
    }
}

const NUM_NUTRIENT_ADDITIONS: i32 = 4;

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
