use sea_orm::*;
use uuid::Uuid;

use crate::models::entities::hydrometers::{self, ActiveModel, Column, Entity as Hydrometer};
use crate::models::entities::readings::{self, Entity as Reading};
use shared::{CreateHydrometer, HydrometerResponse, TiltColor, TiltReading, UpdateHydrometer};

fn model_to_response(model: hydrometers::Model, latest: Option<TiltReading>) -> HydrometerResponse {
    HydrometerResponse {
        id: model.id,
        color: TiltColor::parse(&model.color).unwrap_or(TiltColor::Red),
        name: model.name,
        temp_offset_f: model.temp_offset_f,
        gravity_offset: model.gravity_offset,
        created_at: model.created_at.into(),
        latest_reading: latest,
    }
}

async fn latest_reading_for(
    db: &DatabaseConnection,
    hydrometer_id: Uuid,
    color: &TiltColor,
) -> Option<TiltReading> {
    let reading = Reading::find()
        .filter(readings::Column::HydrometerId.eq(hydrometer_id))
        .order_by_desc(readings::Column::RecordedAt)
        .one(db)
        .await
        .ok()??;
    Some(TiltReading::new(
        *color,
        reading.temperature_f,
        reading.gravity,
        reading.rssi,
        reading.recorded_at.into(),
    ))
}

pub async fn find_all(db: &DatabaseConnection) -> Result<Vec<HydrometerResponse>, DbErr> {
    let models = Hydrometer::find().all(db).await?;
    let mut results = Vec::with_capacity(models.len());
    for model in models {
        let color = TiltColor::parse(&model.color).unwrap_or(TiltColor::Red);
        let latest = latest_reading_for(db, model.id, &color).await;
        results.push(model_to_response(model, latest));
    }
    Ok(results)
}

pub async fn find_by_id(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<HydrometerResponse>, DbErr> {
    let Some(model) = Hydrometer::find_by_id(id).one(db).await? else {
        return Ok(None);
    };
    let color = TiltColor::parse(&model.color).unwrap_or(TiltColor::Red);
    let latest = latest_reading_for(db, model.id, &color).await;
    Ok(Some(model_to_response(model, latest)))
}

pub async fn create(
    db: &DatabaseConnection,
    input: CreateHydrometer,
) -> Result<HydrometerResponse, DbErr> {
    let model = ActiveModel {
        id: Set(Uuid::new_v4()),
        color: Set(format!("{:?}", input.color)),
        name: Set(input.name),
        temp_offset_f: Set(0.0),
        gravity_offset: Set(0.0),
        created_at: Set(chrono::Utc::now().into()),
    };
    let result = Hydrometer::insert(model).exec_with_returning(db).await?;
    Ok(model_to_response(result, None))
}

pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    input: UpdateHydrometer,
) -> Result<Option<HydrometerResponse>, DbErr> {
    let existing = Hydrometer::find_by_id(id).one(db).await?;
    let Some(existing) = existing else {
        return Ok(None);
    };

    let mut active: ActiveModel = existing.into();
    if let Some(name) = input.name {
        active.name = Set(Some(name));
    }
    if let Some(temp_offset_f) = input.temp_offset_f {
        active.temp_offset_f = Set(temp_offset_f);
    }
    if let Some(gravity_offset) = input.gravity_offset {
        active.gravity_offset = Set(gravity_offset);
    }

    let updated = active.update(db).await?;
    let color = TiltColor::parse(&updated.color).unwrap_or(TiltColor::Red);
    let latest = latest_reading_for(db, updated.id, &color).await;
    Ok(Some(model_to_response(updated, latest)))
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<bool, DbErr> {
    let result = Hydrometer::delete_by_id(id).exec(db).await?;
    Ok(result.rows_affected > 0)
}

pub async fn find_by_color(
    db: &DatabaseConnection,
    color: &shared::TiltColor,
) -> Result<Option<hydrometers::Model>, DbErr> {
    Hydrometer::find()
        .filter(Column::Color.eq(format!("{:?}", color)))
        .one(db)
        .await
}

pub async fn find_or_create_by_color(
    db: &DatabaseConnection,
    color: &shared::TiltColor,
) -> Result<hydrometers::Model, DbErr> {
    if let Some(existing) = find_by_color(db, color).await? {
        return Ok(existing);
    }
    let model = ActiveModel {
        id: Set(Uuid::new_v4()),
        color: Set(format!("{:?}", color)),
        name: Set(None),
        temp_offset_f: Set(0.0),
        gravity_offset: Set(0.0),
        created_at: Set(chrono::Utc::now().into()),
    };
    let result = Hydrometer::insert(model).exec_with_returning(db).await?;
    tracing::info!(color = ?color, id = %result.id, "Auto-registered new hydrometer");
    Ok(result)
}
