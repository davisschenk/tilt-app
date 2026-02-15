use sea_orm::*;
use uuid::Uuid;

use crate::models::entities::brews::{self, ActiveModel, Column, Entity as Brew};
use crate::models::entities::hydrometers::Entity as Hydrometer;
use crate::models::entities::readings::{self, Entity as Reading};
use shared::{BrewResponse, BrewStatus, CreateBrew, TiltColor, TiltReading, UpdateBrew};

fn model_to_response(model: brews::Model, latest: Option<TiltReading>) -> BrewResponse {
    let status = match model.status.as_str() {
        "Completed" => BrewStatus::Completed,
        "Archived" => BrewStatus::Archived,
        _ => BrewStatus::Active,
    };
    BrewResponse {
        id: model.id,
        name: model.name,
        style: model.style,
        og: model.og,
        fg: model.fg,
        target_fg: model.target_fg,
        abv: model.abv,
        status,
        start_date: model.start_date.map(Into::into),
        end_date: model.end_date.map(Into::into),
        notes: model.notes,
        hydrometer_id: model.hydrometer_id,
        created_at: model.created_at.into(),
        updated_at: model.updated_at.into(),
        latest_reading: latest,
    }
}

async fn latest_reading_for(
    db: &DatabaseConnection,
    brew: &brews::Model,
) -> Option<TiltReading> {
    let hydrometer = Hydrometer::find_by_id(brew.hydrometer_id)
        .one(db)
        .await
        .ok()??;
    let color = TiltColor::parse(&hydrometer.color).unwrap_or(TiltColor::Red);
    let reading = Reading::find()
        .filter(readings::Column::BrewId.eq(brew.id))
        .order_by_desc(readings::Column::RecordedAt)
        .one(db)
        .await
        .ok()??;
    Some(TiltReading::new(
        color,
        reading.temperature_f,
        reading.gravity,
        reading.rssi,
        reading.recorded_at.into(),
    ))
}

pub async fn find_all(
    db: &DatabaseConnection,
    status_filter: Option<&str>,
) -> Result<Vec<BrewResponse>, DbErr> {
    let mut query = Brew::find();
    if let Some(status) = status_filter {
        query = query.filter(Column::Status.eq(status));
    }
    let models = query.all(db).await?;
    let mut results = Vec::with_capacity(models.len());
    for model in models {
        let latest = latest_reading_for(db, &model).await;
        results.push(model_to_response(model, latest));
    }
    Ok(results)
}

pub async fn find_by_id(db: &DatabaseConnection, id: Uuid) -> Result<Option<BrewResponse>, DbErr> {
    let Some(model) = Brew::find_by_id(id).one(db).await? else {
        return Ok(None);
    };
    let latest = latest_reading_for(db, &model).await;
    Ok(Some(model_to_response(model, latest)))
}

pub async fn create(db: &DatabaseConnection, input: CreateBrew) -> Result<BrewResponse, DbErr> {
    let now = chrono::Utc::now();
    let model = ActiveModel {
        id: Set(Uuid::new_v4()),
        name: Set(input.name),
        style: Set(input.style),
        og: Set(input.og),
        fg: Set(None),
        target_fg: Set(input.target_fg),
        abv: Set(None),
        status: Set("Active".to_string()),
        start_date: Set(Some(now.into())),
        end_date: Set(None),
        notes: Set(input.notes),
        hydrometer_id: Set(input.hydrometer_id),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
    };
    let result = Brew::insert(model).exec_with_returning(db).await?;
    Ok(model_to_response(result, None))
}

pub async fn update(
    db: &DatabaseConnection,
    id: Uuid,
    input: UpdateBrew,
) -> Result<Option<BrewResponse>, DbErr> {
    let existing = Brew::find_by_id(id).one(db).await?;
    let Some(existing) = existing else {
        return Ok(None);
    };

    let mut active: ActiveModel = existing.into();
    if let Some(name) = input.name {
        active.name = Set(name);
    }
    if let Some(style) = input.style {
        active.style = Set(Some(style));
    }
    if let Some(og) = input.og {
        active.og = Set(Some(og));
    }
    if let Some(fg) = input.fg {
        active.fg = Set(Some(fg));
    }
    if let Some(target_fg) = input.target_fg {
        active.target_fg = Set(Some(target_fg));
    }
    if let Some(abv) = input.abv {
        active.abv = Set(Some(abv));
    }
    if let Some(status) = input.status {
        active.status = Set(format!("{:?}", status));
    }
    if let Some(notes) = input.notes {
        active.notes = Set(if notes.is_empty() { None } else { Some(notes) });
    }
    if let Some(end_date) = input.end_date {
        active.end_date = Set(Some(end_date.into()));
    }
    active.updated_at = Set(chrono::Utc::now().into());

    let updated = active.update(db).await?;
    let latest = latest_reading_for(db, &updated).await;
    Ok(Some(model_to_response(updated, latest)))
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<bool, DbErr> {
    let result = Brew::delete_by_id(id).exec(db).await?;
    Ok(result.rows_affected > 0)
}

pub async fn find_active_for_hydrometer(
    db: &DatabaseConnection,
    hydrometer_id: Uuid,
) -> Result<Option<brews::Model>, DbErr> {
    Brew::find()
        .filter(Column::HydrometerId.eq(hydrometer_id))
        .filter(Column::Status.eq("Active"))
        .one(db)
        .await
}
