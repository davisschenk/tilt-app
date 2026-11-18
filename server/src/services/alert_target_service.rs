use sea_orm::*;
use uuid::Uuid;

use crate::models::entities::alert_targets::{self, ActiveModel, Column, Entity as AlertTarget};
use crate::services::{WriteError, limits};
use shared::{AlertTargetResponse, CreateAlertTarget, UpdateAlertTarget, WebhookFormat};

fn model_to_response(model: alert_targets::Model) -> AlertTargetResponse {
    let format =
        serde_json::from_value::<WebhookFormat>(serde_json::Value::String(model.format.clone()))
            .unwrap_or(WebhookFormat::GenericJson);

    AlertTargetResponse {
        id: model.id,
        name: model.name,
        url: model.url,
        format,
        secret_header: model.secret_header,
        enabled: model.enabled,
        created_at: model.created_at.into(),
        updated_at: model.updated_at.into(),
    }
}

pub async fn find_all(
    db: &DatabaseConnection,
    owner: Uuid,
) -> Result<Vec<AlertTargetResponse>, DbErr> {
    let models = AlertTarget::find()
        .filter(Column::OwnerId.eq(owner))
        .all(db)
        .await?;
    Ok(models.into_iter().map(model_to_response).collect())
}

/// Fetch an alert target only if `owner` owns it. Other users' ids look like misses.
pub async fn find_raw_by_id(
    db: &DatabaseConnection,
    owner: Uuid,
    id: Uuid,
) -> Result<Option<alert_targets::Model>, DbErr> {
    AlertTarget::find()
        .filter(Column::Id.eq(id))
        .filter(Column::OwnerId.eq(owner))
        .one(db)
        .await
}

pub async fn find_by_id(
    db: &DatabaseConnection,
    owner: Uuid,
    id: Uuid,
) -> Result<Option<AlertTargetResponse>, DbErr> {
    Ok(find_raw_by_id(db, owner, id).await?.map(model_to_response))
}

pub async fn create(
    db: &DatabaseConnection,
    owner: Uuid,
    input: CreateAlertTarget,
) -> Result<AlertTargetResponse, WriteError> {
    if AlertTarget::find()
        .filter(Column::OwnerId.eq(owner))
        .count(db)
        .await?
        >= limits::MAX_ALERT_TARGETS
    {
        return Err(WriteError::LimitReached("alert targets"));
    }
    let now = chrono::Utc::now().into();
    let format_str = serde_json::to_value(input.format).map_or_else(
        |_| "generic_json".to_string(),
        |v| v.as_str().unwrap_or("generic_json").to_string(),
    );

    let model = ActiveModel {
        id: Set(Uuid::new_v4()),
        owner_id: Set(owner),
        name: Set(input.name),
        url: Set(input.url),
        format: Set(format_str),
        secret_header: Set(input.secret_header),
        enabled: Set(input.enabled.unwrap_or(true)),
        created_at: Set(now),
        updated_at: Set(now),
    };
    let result = AlertTarget::insert(model).exec_with_returning(db).await?;
    Ok(model_to_response(result))
}

pub async fn update(
    db: &DatabaseConnection,
    owner: Uuid,
    id: Uuid,
    input: UpdateAlertTarget,
) -> Result<Option<AlertTargetResponse>, DbErr> {
    let Some(existing) = find_raw_by_id(db, owner, id).await? else {
        return Ok(None);
    };

    let mut active: ActiveModel = existing.into();

    if let Some(name) = input.name {
        active.name = Set(name);
    }
    if let Some(url) = input.url {
        active.url = Set(url);
    }
    if let Some(format) = input.format {
        let format_str = serde_json::to_value(format).map_or_else(
            |_| "generic_json".to_string(),
            |v| v.as_str().unwrap_or("generic_json").to_string(),
        );
        active.format = Set(format_str);
    }
    if let Some(secret_header) = input.secret_header {
        active.secret_header = Set(Some(secret_header));
    }
    if let Some(enabled) = input.enabled {
        active.enabled = Set(enabled);
    }
    active.updated_at = Set(chrono::Utc::now().into());

    let updated = active.update(db).await?;
    Ok(Some(model_to_response(updated)))
}

pub async fn delete(db: &DatabaseConnection, owner: Uuid, id: Uuid) -> Result<bool, DbErr> {
    let result = AlertTarget::delete_many()
        .filter(Column::Id.eq(id))
        .filter(Column::OwnerId.eq(owner))
        .exec(db)
        .await?;
    Ok(result.rows_affected > 0)
}
