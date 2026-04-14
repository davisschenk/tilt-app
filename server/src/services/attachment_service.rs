use chrono::Utc;
use sea_orm::*;
use uuid::Uuid;

use crate::models::entities::event_attachments::{
    self, ActiveModel, Column, Entity as EventAttachment,
};
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

pub async fn find_raw_by_id(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<event_attachments::Model>, DbErr> {
    EventAttachment::find_by_id(id).one(db).await
}

pub async fn delete(db: &DatabaseConnection, id: Uuid) -> Result<bool, DbErr> {
    let result = EventAttachment::delete_by_id(id).exec(db).await?;
    Ok(result.rows_affected > 0)
}
