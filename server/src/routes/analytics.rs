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

    let expected_interval_minutes = 15.0_f64;
    let threshold_minutes = expected_interval_minutes * 3.0;

    // Build parallel arrays: timestamps and DateTime values
    let reading_times: Vec<chrono::DateTime<Utc>> =
        all_readings.iter().map(|r| r.recorded_at.into()).collect();
    let reading_hours: Vec<f64> = {
        let first = reading_times.first().copied().unwrap_or(Utc::now());
        reading_times
            .iter()
            .map(|t| (*t - first).num_seconds() as f64 / 3600.0)
            .collect()
    };
    let raw_gaps = analytics_service::find_reading_gaps(&reading_hours, threshold_minutes);
    let gaps: Vec<shared::ReadingGap> = raw_gaps
        .into_iter()
        .filter_map(|(start_h, end_h)| {
            // Find the matching DateTime values by index lookup
            let start_idx = reading_hours
                .iter()
                .position(|&h| (h - start_h).abs() < 1e-9)?;
            let end_idx = reading_hours
                .iter()
                .position(|&h| (h - end_h).abs() < 1e-9)?;
            let start_at = reading_times[start_idx];
            let end_at = reading_times[end_idx];
            let duration_minutes = (end_at - start_at).num_seconds() as f64 / 60.0;
            Some(shared::ReadingGap {
                start_at,
                end_at,
                duration_minutes,
            })
        })
        .collect();

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
