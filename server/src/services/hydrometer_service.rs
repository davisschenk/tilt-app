use sea_orm::*;
use uuid::Uuid;

use crate::models::entities::hydrometers::{self, ActiveModel, Column, Entity as Hydrometer};
use shared::{CreateHydrometer, HydrometerResponse, UpdateHydrometer};

impl From<hydrometers::Model> for HydrometerResponse {
    fn from(model: hydrometers::Model) -> Self {
        Self {
            id: model.id,
            color: shared::TiltColor::from_str(&model.color)
                .unwrap_or(shared::TiltColor::Red),
            name: model.name,
            temp_offset_f: model.temp_offset_f,
            gravity_offset: model.gravity_offset,
            created_at: model.created_at.into(),
        }
    }
}

pub async fn find_all(db: &DatabaseConnection) -> Result<Vec<HydrometerResponse>, DbErr> {
    let models = Hydrometer::find().all(db).await?;
    Ok(models.into_iter().map(HydrometerResponse::from).collect())
}

pub async fn find_by_id(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<HydrometerResponse>, DbErr> {
    let model = Hydrometer::find_by_id(id).one(db).await?;
    Ok(model.map(HydrometerResponse::from))
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
    Ok(HydrometerResponse::from(result))
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
    Ok(Some(HydrometerResponse::from(updated)))
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
