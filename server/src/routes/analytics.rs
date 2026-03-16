use chrono::Utc;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Route, State, get, routes};
use sea_orm::*;
use uuid::Uuid;

use shared::BrewAnalytics;

use crate::guards::current_user::CurrentUser;
use crate::models::entities::brews::Entity as Brew;
use crate::models::entities::hydrometers::Entity as Hydrometer;
use crate::models::entities::readings::{self, Entity as Reading};
use crate::services::analytics_service::{self, GravityPoint};
use shared::TiltColor;

const ANALYTICS_WINDOW: u32 = 168;

#[get("/brews/<brew_id>/analytics")]
pub async fn brew_analytics(
    _user: CurrentUser,
    db: &State<DatabaseConnection>,
    brew_id: &str,
) -> Result<Json<BrewAnalytics>, Status> {
    let brew_id = Uuid::parse_str(brew_id).map_err(|_| Status::UnprocessableEntity)?;

    let brew = Brew::find_by_id(brew_id)
        .one(db.inner())
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;

    let since = Utc::now() - chrono::Duration::hours(ANALYTICS_WINDOW as i64);

    let all_readings = Reading::find()
        .filter(readings::Column::BrewId.eq(brew_id))
        .order_by_asc(readings::Column::RecordedAt)
        .all(db.inner())
        .await
        .map_err(|_| Status::InternalServerError)?;

    let latest = all_readings.last();

    let current_gravity = latest.map(|r| r.gravity);
    let current_temp_f = latest.map(|r| r.temperature_f);
    let last_reading_at = latest.map(|r| {
        let dt: chrono::DateTime<Utc> = r.recorded_at.into();
        dt
    });

    let (live_abv, apparent_attenuation) = match (brew.og, current_gravity) {
        (Some(og), Some(g)) => (
            Some(analytics_service::compute_live_abv(og, g)),
            Some(analytics_service::compute_apparent_attenuation(og, g)),
        ),
        _ => (None, None),
    };

    let window_readings: Vec<_> = all_readings
        .iter()
        .filter(|r| {
            let dt: chrono::DateTime<Utc> = r.recorded_at.into();
            dt >= since
        })
        .collect();

    let (predicted_fg_date, hours_remaining) = if let Some(target_fg) = brew.target_fg {
        if window_readings.len() >= 3 {
            let first_time: chrono::DateTime<Utc> = window_readings[0].recorded_at.into();
            let points: Vec<GravityPoint> = window_readings
                .iter()
                .map(|r| {
                    let dt: chrono::DateTime<Utc> = r.recorded_at.into();
                    let hours = (dt - first_time).num_seconds() as f64 / 3600.0;
                    GravityPoint {
                        hours,
                        gravity: r.gravity,
                    }
                })
                .collect();
            let now = Utc::now();
            let predicted = analytics_service::predict_fg_date(&points, target_fg, now);
            let hrs = predicted.map(|p| (p - now).num_seconds() as f64 / 3600.0);
            (predicted, hrs)
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    let expected_interval = 15.0_f64;
    let threshold = expected_interval * 3.0;
    let mut gaps = Vec::new();
    for window in all_readings.windows(2) {
        let a: chrono::DateTime<Utc> = window[0].recorded_at.into();
        let b: chrono::DateTime<Utc> = window[1].recorded_at.into();
        let diff_minutes = (b - a).num_seconds() as f64 / 60.0;
        if diff_minutes > threshold {
            gaps.push(shared::ReadingGap {
                start_at: a,
                end_at: b,
                duration_minutes: diff_minutes,
            });
        }
    }

    let _ = Hydrometer::find_by_id(brew.hydrometer_id)
        .one(db.inner())
        .await
        .ok()
        .flatten()
        .and_then(|h| TiltColor::parse(&h.color));

    Ok(Json(BrewAnalytics {
        current_gravity,
        current_temp_f,
        last_reading_at,
        live_abv,
        apparent_attenuation,
        predicted_fg_date,
        hours_remaining,
        gaps,
    }))
}

pub fn routes() -> Vec<Route> {
    routes![brew_analytics]
}
