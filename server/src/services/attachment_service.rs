use chrono::Utc;
use sea_orm::*;
use uuid::Uuid;

use crate::models::entities::brew_events::Entity as BrewEvent;
use crate::models::entities::event_attachments::{
    self, ActiveModel, Column, Entity as EventAttachment,
};
use crate::services::brew_service;
use shared::EventAttachmentResponse;

fn model_to_response(model: event_attachments::Model) -> EventAttachmentResponse {
    let url = format!("/api/v1/attachments/{}/file", model.id);
    EventAttachmentResponse {
        id: model.id,
        event_id: model.event_id,
        filename: model.filename,
        content_type: model.content_type,
        size_bytes: model.size_bytes,
        created_at: model.created_at.into(),
        url,
    }
}

pub async fn create(
    db: &DatabaseConnection,
    event_id: Uuid,
    filename: String,
    storage_path: String,
    content_type: String,
    size_bytes: i64,
) -> Result<EventAttachmentResponse, DbErr> {
    let model = ActiveModel {
        id: Set(Uuid::new_v4()),
        event_id: Set(event_id),
        filename: Set(filename),
        storage_path: Set(storage_path),
        content_type: Set(content_type),
        size_bytes: Set(size_bytes),
        created_at: Set(Utc::now().fixed_offset()),
    };
    let inserted = model.insert(db).await?;
    Ok(model_to_response(inserted))
}

pub async fn find_by_event(
    db: &DatabaseConnection,
    event_id: Uuid,
) -> Result<Vec<EventAttachmentResponse>, DbErr> {
    EventAttachment::find()
        .filter(Column::EventId.eq(event_id))
        .order_by_asc(Column::CreatedAt)
        .all(db)
        .await
        .map(|models| models.into_iter().map(model_to_response).collect())
}

/// Fetch an attachment only if `owner` owns the brew its event belongs to.
/// Other users' ids look like misses.
pub async fn find_owned_raw(
    db: &DatabaseConnection,
    owner: Uuid,
    id: Uuid,
) -> Result<Option<event_attachments::Model>, DbErr> {
    let Some(att) = EventAttachment::find_by_id(id).one(db).await? else {
        return Ok(None);
    };
    let Some(event) = BrewEvent::find_by_id(att.event_id).one(db).await? else {
        return Ok(None);
    };
    if brew_service::find_owned(db, owner, event.brew_id)
        .await?
        .is_none()
    {
        return Ok(None);
    }
    Ok(Some(att))
}

/// Total bytes of attachments across every brew `owner` owns (for quotas).
pub async fn total_bytes_for_owner(db: &DatabaseConnection, owner: Uuid) -> Result<i64, DbErr> {
    use crate::models::entities::{brew_events, brews};
    use sea_orm::sea_query::Query;

    let owned_events = Query::select()
        .column(brew_events::Column::Id)
        .from(brew_events::Entity)
        .and_where(
            brew_events::Column::BrewId.in_subquery(
                Query::select()
                    .column(brews::Column::Id)
                    .from(brews::Entity)
                    .and_where(brews::Column::OwnerId.eq(owner))
                    .to_owned(),
            ),
        )
        .to_owned();
    let sizes: Vec<i64> = EventAttachment::find()
        .select_only()
        .column(Column::SizeBytes)
        .filter(Column::EventId.in_subquery(owned_events))
        .into_tuple()
        .all(db)
        .await?;
    Ok(sizes.iter().sum())
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<bool, DbErr> {
    let result = EventAttachment::delete_by_id(id).exec(db).await?;
    Ok(result.rows_affected > 0)
}
