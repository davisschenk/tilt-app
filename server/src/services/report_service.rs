//! Public brew reports: an owner publishes a fixed snapshot of selected batch
//! content behind an unguessable token, and can refresh or revoke it.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use sea_orm::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::entities::{
    brew_reports::{self, ActiveModel, Column, Entity as Report},
    brews, event_attachments,
};
use crate::services::preferences_service::Preferences;
use crate::services::{
    WriteError, analytics_service, attachment_service, brew_event_service, limits, reading_service,
};

/// Length of a report token (two simple UUIDs).
const TOKEN_LEN: usize = 64;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportOptions {
    pub event_ids: Vec<Uuid>,
    pub include_notes: bool,
    pub include_photos: bool,
    pub units: Preferences,
}

/// The published snapshot. Field names are what `web/src/pages/brew-report.tsx`
/// reads, so changes here are API changes.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportPayload {
    name: String,
    style: Option<String>,
    status: String,
    og: Option<f64>,
    fg: Option<f64>,
    target_fg: Option<f64>,
    abv: Option<f64>,
    yeast_strain: Option<String>,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
    notes: Option<String>,
    units: Preferences,
    events: Vec<ReportEvent>,
    readings: Vec<ReportReading>,
    created_at: DateTime<Utc>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportEvent {
    label: String,
    event_type: String,
    event_time: DateTime<Utc>,
    gravity_at_event: Option<f64>,
    temp_at_event: Option<f64>,
    notes: Option<String>,
    photos: Vec<ReportPhoto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportPhoto {
    id: Uuid,
    filename: String,
    url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReportReading {
    recorded_at: DateTime<Utc>,
    gravity: f64,
    temperature_f: f64,
}

#[derive(Serialize)]
pub struct ReportLink {
    pub id: Uuid,
    pub token: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportSummary {
    pub id: Uuid,
    pub token: String,
    pub created_at: sea_orm::prelude::DateTimeWithTimeZone,
}

fn new_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

/// Build the snapshot's log entries. Errors with `BadReference` if any
/// requested id is not an event of this brew.
async fn selected_events(
    db: &DatabaseConnection,
    brew_id: Uuid,
    options: &ReportOptions,
    token: &str,
) -> Result<Vec<ReportEvent>, WriteError> {
    let wanted: HashSet<Uuid> = options.event_ids.iter().copied().collect();
    if wanted.is_empty() {
        return Ok(Vec::new());
    }
    let events =
        brew_event_service::find_in_brew_by_ids_raw(db, brew_id, wanted.iter().copied()).await?;
    if events.len() != wanted.len() {
        return Err(WriteError::BadReference);
    }

    let mut photos: HashMap<Uuid, Vec<event_attachments::Model>> = HashMap::new();
    if options.include_photos {
        for a in attachment_service::find_by_events_raw(db, wanted.iter().copied()).await? {
            photos.entry(a.event_id).or_default().push(a);
        }
    }

    Ok(events
        .into_iter()
        .map(|e| ReportEvent {
            photos: photos
                .remove(&e.id)
                .unwrap_or_default()
                .into_iter()
                .map(|a| ReportPhoto {
                    url: format!("/api/v1/reports/{token}/photos/{}", a.id),
                    id: a.id,
                    filename: a.filename,
                })
                .collect(),
            label: e.label,
            event_type: e.event_type,
            event_time: e.event_time.into(),
            gravity_at_event: e.gravity_at_event,
            temp_at_event: e.temp_at_event,
            notes: e.notes.filter(|_| options.include_notes),
        })
        .collect())
}

/// Publish a new snapshot of `brew`, or refresh report `replace` in place
/// (keeping its link). The caller must own `brew`.
pub async fn publish(
    db: &DatabaseConnection,
    owner: Uuid,
    brew: brews::Model,
    options: &ReportOptions,
    replace: Option<Uuid>,
) -> Result<ReportLink, WriteError> {
    let previous = match replace {
        Some(report_id) => Some(
            Report::find_by_id(report_id)
                .filter(Column::OwnerId.eq(owner))
                .filter(Column::BrewId.eq(brew.id))
                .one(db)
                .await?
                .ok_or(WriteError::NotFound)?,
        ),
        None => {
            let count = Report::find()
                .filter(Column::BrewId.eq(brew.id))
                .count(db)
                .await?;
            if count >= limits::MAX_REPORTS_PER_BREW {
                return Err(WriteError::LimitReached("reports"));
            }
            None
        }
    };

    let (id, token) = previous
        .as_ref()
        .map(|r| (r.id, r.token.clone()))
        .unwrap_or_else(|| (Uuid::new_v4(), new_token()));

    let events = selected_events(db, brew.id, options, &token).await?;
    let readings = reading_service::find_filtered(
        db,
        owner,
        &shared::ReadingsQuery {
            brew_id: Some(brew.id),
            hydrometer_id: None,
            since: None,
            until: None,
            limit: None,
        },
    )
    .await?
    .into_iter()
    .map(|r| ReportReading {
        recorded_at: r.recorded_at,
        gravity: r.gravity,
        temperature_f: r.temperature_f,
    })
    .collect();

    let now = Utc::now();
    let payload = ReportPayload {
        abv: brew
            .og
            .zip(brew.fg)
            .map(|(og, fg)| analytics_service::compute_abv(og, fg)),
        name: brew.name,
        style: brew.style,
        status: brew.status,
        og: brew.og,
        fg: brew.fg,
        target_fg: brew.target_fg,
        yeast_strain: brew.yeast_strain,
        start_date: brew.start_date.map(Into::into),
        end_date: brew.end_date.map(Into::into),
        notes: brew.notes.filter(|_| options.include_notes),
        units: options.units.clone(),
        events,
        readings,
        created_at: now,
    };

    let model = ActiveModel {
        id: Set(id),
        owner_id: Set(owner),
        brew_id: Set(brew.id),
        token: Set(token.clone()),
        payload: Set(serde_json::to_value(payload)
            .map_err(|e| DbErr::Custom(format!("report payload: {e}")))?),
        created_at: Set(now.into()),
    };
    if previous.is_some() {
        model.update(db).await?;
    } else {
        model.insert(db).await?;
    }
    Ok(ReportLink { id, token })
}

pub async fn list(
    db: &DatabaseConnection,
    owner: Uuid,
    brew_id: Uuid,
) -> Result<Vec<ReportSummary>, DbErr> {
    Ok(Report::find()
        .filter(Column::BrewId.eq(brew_id))
        .filter(Column::OwnerId.eq(owner))
        .order_by_desc(Column::CreatedAt)
        .all(db)
        .await?
        .into_iter()
        .map(|r| ReportSummary {
            id: r.id,
            token: r.token,
            created_at: r.created_at,
        })
        .collect())
}

/// Every report the owner has published, oldest first (for account export).
pub async fn find_all(
    db: &DatabaseConnection,
    owner: Uuid,
) -> Result<Vec<brew_reports::Model>, DbErr> {
    Report::find()
        .filter(Column::OwnerId.eq(owner))
        .order_by_asc(Column::CreatedAt)
        .all(db)
        .await
}

/// Revoke a report. Returns whether one was deleted.
pub async fn revoke(
    db: &DatabaseConnection,
    owner: Uuid,
    brew_id: Uuid,
    id: Uuid,
) -> Result<bool, DbErr> {
    let result = Report::delete_many()
        .filter(Column::Id.eq(id))
        .filter(Column::BrewId.eq(brew_id))
        .filter(Column::OwnerId.eq(owner))
        .exec(db)
        .await?;
    Ok(result.rows_affected > 0)
}

/// Look up a published report by its public token.
pub async fn find_by_token(
    db: &DatabaseConnection,
    token: &str,
) -> Result<Option<brew_reports::Model>, DbErr> {
    if token.len() != TOKEN_LEN {
        return Ok(None);
    }
    Report::find().filter(Column::Token.eq(token)).one(db).await
}

/// A photo the report at `token` publishes, or `None` if the report does not
/// include it (or does not exist).
pub async fn find_public_photo(
    db: &DatabaseConnection,
    token: &str,
    photo_id: Uuid,
) -> Result<Option<event_attachments::Model>, DbErr> {
    let Some(report) = find_by_token(db, token).await? else {
        return Ok(None);
    };
    let id = photo_id.to_string();
    let published = report.payload["events"].as_array().is_some_and(|events| {
        events.iter().any(|e| {
            e["photos"]
                .as_array()
                .is_some_and(|photos| photos.iter().any(|p| p["id"] == id.as_str()))
        })
    });
    if !published {
        return Ok(None);
    }
    event_attachments::Entity::find_by_id(photo_id)
        .one(db)
        .await
}
