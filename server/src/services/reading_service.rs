use sea_orm::*;
use uuid::Uuid;

use crate::models::entities::hydrometers::Entity as Hydrometer;
use crate::models::entities::readings::{self, ActiveModel, Column, Entity as Reading};
use shared::{ReadingResponse, ReadingsQuery, TiltColor, TiltReading};

fn model_to_response(model: readings::Model, color: TiltColor) -> ReadingResponse {
    ReadingResponse {
        id: model.id,
        brew_id: model.brew_id,
        hydrometer_id: model.hydrometer_id,
        color,
        temperature_f: model.temperature_f,
        gravity: model.gravity,
        rssi: model.rssi,
        recorded_at: model.recorded_at.into(),
        created_at: model.created_at.into(),
    }
}

pub async fn batch_create(
    db: &DatabaseConnection,
    readings: Vec<TiltReading>,
    hydrometer_id: Uuid,
    brew_id: Option<Uuid>,
) -> Result<u64, DbErr> {
    if readings.is_empty() {
        return Ok(0);
    }

    let models: Vec<ActiveModel> = readings
        .into_iter()
        .map(|r| ActiveModel {
            id: Set(Uuid::new_v4()),
            brew_id: Set(brew_id),
            hydrometer_id: Set(hydrometer_id),
            temperature_f: Set(r.temperature_f),
            gravity: Set(r.gravity),
            rssi: Set(r.rssi),
            recorded_at: Set(r.recorded_at.into()),
            created_at: Set(chrono::Utc::now().into()),
        })
        .collect();

    let count = models.len() as u64;
    Reading::insert_many(models).exec(db).await?;
    Ok(count)
}

pub async fn find_filtered(
    db: &DatabaseConnection,
    query: &ReadingsQuery,
) -> Result<Vec<ReadingResponse>, DbErr> {
    let mut select = Reading::find();

    if let Some(brew_id) = query.brew_id {
        select = select.filter(Column::BrewId.eq(brew_id));
    }
    if let Some(hydrometer_id) = query.hydrometer_id {
        select = select.filter(Column::HydrometerId.eq(hydrometer_id));
    }
    if let Some(since) = query.since {
        let since_tz: chrono::DateTime<chrono::FixedOffset> = since.into();
        select = select.filter(Column::RecordedAt.gte(since_tz));
    }
    if let Some(until) = query.until {
        let until_tz: chrono::DateTime<chrono::FixedOffset> = until.into();
        select = select.filter(Column::RecordedAt.lte(until_tz));
    }

    let limit = query.limit_or_default();
    let models = select
        .order_by_desc(Column::RecordedAt)
        .limit(limit)
        .all(db)
        .await?;

    // Build a hydrometer_id -> TiltColor lookup
    let hydro_ids: Vec<Uuid> = models.iter().map(|m| m.hydrometer_id).collect::<std::collections::HashSet<_>>().into_iter().collect();
    let hydrometers = Hydrometer::find()
        .filter(crate::models::entities::hydrometers::Column::Id.is_in(hydro_ids))
        .all(db)
        .await?;
    let color_map: std::collections::HashMap<Uuid, TiltColor> = hydrometers
        .into_iter()
        .map(|h| (h.id, TiltColor::parse(&h.color).unwrap_or(TiltColor::Red)))
        .collect();

    Ok(models
        .into_iter()
        .map(|m| {
            let color = color_map.get(&m.hydrometer_id).copied().unwrap_or(TiltColor::Red);
            model_to_response(m, color)
        })
        .collect())
}
