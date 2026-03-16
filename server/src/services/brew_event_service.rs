use chrono::{DateTime, Utc};
use sea_orm::*;
use uuid::Uuid;

use crate::models::entities::brew_events::{self, ActiveModel, Column, Entity as BrewEvent};
use shared::{BrewEventResponse, BrewEventType, CreateBrewEvent, UpdateBrewEvent};

fn parse_event_type(s: &str) -> BrewEventType {
    serde_json::from_value::<BrewEventType>(serde_json::Value::String(s.to_string()))
        .unwrap_or(BrewEventType::Note)
}

fn enum_to_string<T: serde::Serialize>(val: T) -> String {
    serde_json::to_value(val)
        .ok()
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default()
}

fn model_to_response(model: brew_events::Model) -> BrewEventResponse {
    BrewEventResponse {
        id: model.id,
        brew_id: model.brew_id,
        event_type: parse_event_type(&model.event_type),
        label: model.label,
        notes: model.notes,
        gravity_at_event: model.gravity_at_event,
        temp_at_event: model.temp_at_event,
        event_time: model.event_time.into(),
        created_at: model.created_at.into(),
    }
}

pub async fn find_by_brew(
    db: &DatabaseConnection,
    brew_id: Uuid,
    since: Option<DateTime<Utc>>,
    until: Option<DateTime<Utc>>,
) -> Result<Vec<BrewEventResponse>, DbErr> {
    let mut query = BrewEvent::find().filter(Column::BrewId.eq(brew_id));
    if let Some(s) = since {
        query = query.filter(Column::EventTime.gte(s.fixed_offset()));
    }
    if let Some(u) = until {
        query = query.filter(Column::EventTime.lte(u.fixed_offset()));
    }
    let models = query.order_by_asc(Column::EventTime).all(db).await?;
    Ok(models.into_iter().map(model_to_response).collect())
}

pub async fn find_by_id(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<BrewEventResponse>, DbErr> {
    BrewEvent::find_by_id(id)
        .one(db)
        .await
        .map(|opt| opt.map(model_to_response))
}

pub async fn create(
    db: &DatabaseConnection,
    payload: CreateBrewEvent,
) -> Result<BrewEventResponse, DbErr> {
    let model = ActiveModel {
        id: Set(Uuid::new_v4()),
        brew_id: Set(payload.brew_id),
        event_type: Set(enum_to_string(payload.event_type)),
        label: Set(payload.label),
        notes: Set(payload.notes),
        gravity_at_event: Set(payload.gravity_at_event),
        temp_at_event: Set(payload.temp_at_event),
        event_time: Set(payload.event_time.fixed_offset()),
        created_at: Set(Utc::now().fixed_offset()),
    };
    let inserted = model.insert(db).await?;
    Ok(model_to_response(inserted))
}

pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    payload: UpdateBrewEvent,
) -> Result<Option<BrewEventResponse>, DbErr> {
    let existing = BrewEvent::find_by_id(id).one(db).await?;
    let Some(existing) = existing else {
        return Ok(None);
    };
    let mut model: ActiveModel = existing.into();
    if let Some(label) = payload.label {
        model.label = Set(label);
    }
    if let Some(notes) = payload.notes {
        model.notes = Set(Some(notes));
    }
    if let Some(gravity) = payload.gravity_at_event {
        model.gravity_at_event = Set(Some(gravity));
    }
    if let Some(temp) = payload.temp_at_event {
        model.temp_at_event = Set(Some(temp));
    }
    if let Some(event_time) = payload.event_time {
        model.event_time = Set(event_time.fixed_offset());
    }
    let updated = model.update(db).await?;
    Ok(Some(model_to_response(updated)))
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<bool, DbErr> {
    let result = BrewEvent::delete_by_id(id).exec(db).await?;
    Ok(result.rows_affected > 0)
}
