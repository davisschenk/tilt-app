use sea_orm::*;
use uuid::Uuid;

use crate::models::entities::brews::{self, ActiveModel, Column, Entity as Brew};
use shared::{BrewResponse, BrewStatus, CreateBrew, UpdateBrew};

impl From<brews::Model> for BrewResponse {
    fn from(model: brews::Model) -> Self {
        let status = match model.status.as_str() {
            "Completed" => BrewStatus::Completed,
            "Archived" => BrewStatus::Archived,
            _ => BrewStatus::Active,
        };
        Self {
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
            latest_reading: None,
        }
    }
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
    Ok(models.into_iter().map(BrewResponse::from).collect())
}

pub async fn find_by_id(db: &DatabaseConnection, id: Uuid) -> Result<Option<BrewResponse>, DbErr> {
    let model = Brew::find_by_id(id).one(db).await?;
    Ok(model.map(BrewResponse::from))
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
    Ok(BrewResponse::from(result))
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
        active.notes = Set(Some(notes));
    }
    if let Some(end_date) = input.end_date {
        active.end_date = Set(Some(end_date.into()));
    }
    active.updated_at = Set(chrono::Utc::now().into());

    let updated = active.update(db).await?;
    Ok(Some(BrewResponse::from(updated)))
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
