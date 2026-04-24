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

    let models = if let Some(limit) = query.limit {
        select
            .order_by_asc(Column::RecordedAt)
            .limit(limit)
            .all(db)
            .await?
    } else {
        select.order_by_asc(Column::RecordedAt).all(db).await?
    };

    const DOWNSAMPLE_TARGET: usize = 5_000;
    let models = if models.len() > DOWNSAMPLE_TARGET {
        downsample(models, DOWNSAMPLE_TARGET)
    } else {
        models
    };

    // Build a hydrometer_id -> TiltColor lookup
    let hydro_ids: Vec<Uuid> = models
        .iter()
        .map(|m| m.hydrometer_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
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
            let color = color_map
                .get(&m.hydrometer_id)
                .copied()
                .unwrap_or(TiltColor::Red);
            model_to_response(m, color)
        })
        .collect())
}

fn downsample(mut models: Vec<readings::Model>, target: usize) -> Vec<readings::Model> {
    if models.len() <= target {
        return models;
    }
    models.sort_by_key(|m| m.recorded_at);
    let first = models.first().unwrap().recorded_at.timestamp_millis();
    let last = models.last().unwrap().recorded_at.timestamp_millis();
    let span = (last - first).max(1);
    let bucket_ms = span / target as i64;

    let mut result: Vec<readings::Model> = Vec::with_capacity(target);
    let mut bucket_start = first;

    let mut i = 0;
    while i < models.len() {
        let bucket_end = bucket_start + bucket_ms;
        let mut bucket: Vec<&readings::Model> = Vec::new();
        while i < models.len() && models[i].recorded_at.timestamp_millis() < bucket_end {
            bucket.push(&models[i]);
            i += 1;
        }
        if let Some(mid) = bucket.get(bucket.len() / 2) {
            result.push((*mid).clone());
        }
        bucket_start = bucket_end;
    }
    result
}
