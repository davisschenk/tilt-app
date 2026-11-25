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

/// Readings stored for `owner` since `since` (by arrival time, which a device
/// cannot backdate), for the daily ingest cap.
pub async fn count_since(
    db: &DatabaseConnection,
    owner: Uuid,
    since: chrono::DateTime<chrono::FixedOffset>,
) -> Result<u64, DbErr> {
    Reading::find()
        .filter(
            Column::HydrometerId.in_subquery(
                sea_orm::sea_query::Query::select()
                    .column(crate::models::entities::hydrometers::Column::Id)
                    .from(crate::models::entities::hydrometers::Entity)
                    .and_where(crate::models::entities::hydrometers::Column::OwnerId.eq(owner))
                    .to_owned(),
            ),
        )
        .filter(Column::CreatedAt.gte(since))
        .count(db)
        .await
}

pub async fn find_filtered(
    db: &DatabaseConnection,
    owner: Uuid,
    query: &ReadingsQuery,
) -> Result<Vec<ReadingResponse>, DbErr> {
    // Only readings from hydrometers the caller owns, whatever ids they pass.
    let mut select = Reading::find().filter(
        Column::HydrometerId.in_subquery(
            sea_orm::sea_query::Query::select()
                .column(crate::models::entities::hydrometers::Column::Id)
                .from(crate::models::entities::hydrometers::Entity)
                .and_where(crate::models::entities::hydrometers::Column::OwnerId.eq(owner))
                .to_owned(),
        ),
    );

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

    const DOWNSAMPLE_TARGET: usize = 5_000;
    let models = match query.limit {
        // A caller-supplied limit already bounds the rows we load.
        Some(limit) => {
            let models = select
                .order_by_asc(Column::RecordedAt)
                .limit(limit)
                .all(db)
                .await?;
            if models.len() > DOWNSAMPLE_TARGET {
                downsample(models, DOWNSAMPLE_TARGET)
            } else {
                models
            }
        }
        // Otherwise a long brew could be hundreds of thousands of rows; stream and
        // downsample as they arrive instead of materialising them all.
        None => load_downsampled(db, select, DOWNSAMPLE_TARGET).await?,
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

type TimeBounds = (
    Option<chrono::DateTime<chrono::FixedOffset>>,
    Option<chrono::DateTime<chrono::FixedOffset>>,
);

/// Load matching readings in time order, reduced to about `target` points by
/// taking the middle reading of each equal-width time bucket (the same result as
/// [`downsample`]) while holding only one bucket in memory at a time.
async fn load_downsampled(
    db: &DatabaseConnection,
    select: Select<Reading>,
    target: usize,
) -> Result<Vec<readings::Model>, DbErr> {
    use rocket::futures::TryStreamExt;

    let total = select.clone().count(db).await?;
    if total as usize <= target {
        return select.order_by_asc(Column::RecordedAt).all(db).await;
    }

    let bounds: Option<TimeBounds> = select
        .clone()
        .select_only()
        .column_as(Column::RecordedAt.min(), "first")
        .column_as(Column::RecordedAt.max(), "last")
        .into_tuple()
        .one(db)
        .await?;
    let Some((Some(first), Some(last))) = bounds else {
        return Ok(Vec::new());
    };
    let first_ms = first.timestamp_millis();
    let span = (last.timestamp_millis() - first_ms).max(1);
    // At least 1ms: a zero-width bucket would never advance past rows that share a timestamp.
    let bucket_ms = (span / target.max(1) as i64).max(1);

    let mut stream = select.order_by_asc(Column::RecordedAt).stream(db).await?;
    let mut out: Vec<readings::Model> = Vec::with_capacity(target + 1);
    let mut bucket: Vec<readings::Model> = Vec::new();
    let mut bucket_end = first_ms + bucket_ms;

    let flush = |bucket: &mut Vec<readings::Model>, out: &mut Vec<readings::Model>| {
        if !bucket.is_empty() {
            let mid = bucket.len() / 2;
            out.push(bucket.swap_remove(mid));
            bucket.clear();
        }
    };

    while let Some(m) = stream.try_next().await? {
        let ms = m.recorded_at.timestamp_millis();
        while ms >= bucket_end {
            flush(&mut bucket, &mut out);
            bucket_end += bucket_ms;
        }
        bucket.push(m);
    }
    flush(&mut bucket, &mut out);
    Ok(out)
}

fn downsample(mut models: Vec<readings::Model>, target: usize) -> Vec<readings::Model> {
    if models.len() <= target {
        return models;
    }
    models.sort_by_key(|m| m.recorded_at);
    let (Some(first), Some(last)) = (models.first(), models.last()) else {
        return models;
    };
    let first = first.recorded_at.timestamp_millis();
    let last = last.recorded_at.timestamp_millis();
    let span = (last - first).max(1);
    // At least 1ms so rows sharing a timestamp cannot stall the loop below.
    let bucket_ms = (span / target.max(1) as i64).max(1);

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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn reading(ms: i64) -> readings::Model {
        let t = chrono::Utc
            .timestamp_millis_opt(ms)
            .single()
            .expect("valid test timestamp")
            .fixed_offset();
        readings::Model {
            id: Uuid::new_v4(),
            brew_id: None,
            hydrometer_id: Uuid::nil(),
            temperature_f: 68.0,
            gravity: 1.050,
            rssi: None,
            recorded_at: t,
            created_at: t,
        }
    }

    #[test]
    fn downsample_empty_input_with_zero_target_is_empty() {
        assert!(downsample(Vec::new(), 0).is_empty());
    }

    #[test]
    fn downsample_with_zero_target_does_not_panic() {
        let models: Vec<_> = (0..10).map(|i| reading(i * 1_000)).collect();
        assert!(!downsample(models, 0).is_empty());
    }

    #[test]
    fn downsample_identical_timestamps_terminates() {
        let models: Vec<_> = (0..50).map(|_| reading(1_000)).collect();
        assert_eq!(downsample(models, 5).len(), 1);
    }
}
