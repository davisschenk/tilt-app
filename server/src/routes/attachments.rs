use rocket::fs::TempFile;
use rocket::http::{ContentType, Status};
use rocket::serde::json::Json;
use rocket::{Route, State, delete, get, post, routes};
use sea_orm::DatabaseConnection;
use std::path::PathBuf;
use uuid::Uuid;

use shared::EventAttachmentResponse;

use crate::guards::current_user::CurrentUser;
use crate::services::{attachment_service, brew_event_service};

const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024; // 10 MB

fn upload_dir() -> String {
    std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string())
}

#[post(
    "/brews/<brew_id>/events/<event_id>/attachments",
    data = "<file>",
    format = "multipart/form-data"
)]
async fn upload(
    _user: CurrentUser,
    db: &State<DatabaseConnection>,
    brew_id: &str,
    event_id: &str,
    mut file: TempFile<'_>,
) -> Result<(Status, Json<EventAttachmentResponse>), Status> {
    let _brew_id = Uuid::parse_str(brew_id).map_err(|_| Status::UnprocessableEntity)?;
    let event_id = Uuid::parse_str(event_id).map_err(|_| Status::UnprocessableEntity)?;

    brew_event_service::find_by_id(db.inner(), event_id)
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;

    let content_type = file
        .content_type()
        .map(|ct| ct.to_string())
        .unwrap_or_default();

    if !content_type.starts_with("image/") {
        return Err(Status::UnprocessableEntity);
    }

    let size_bytes = file.len();
    if size_bytes > MAX_FILE_BYTES {
        return Err(Status::PayloadTooLarge);
    }

    let original_name = file
        .raw_name()
        .map(|n| n.dangerous_unsafe_unsanitized_raw().as_str().to_string())
        .unwrap_or_else(|| "upload".to_string());

    let safe_name: String = original_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();

    let attachment_id = Uuid::new_v4();
    let filename_on_disk = format!("{}_{}", attachment_id, safe_name);
    let dir = PathBuf::from(upload_dir()).join(event_id.to_string());
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let dest = dir.join(&filename_on_disk);
    file.persist_to(&dest)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let storage_path = format!("{}/{}", event_id, filename_on_disk);

    let response = attachment_service::create(
        db.inner(),
        event_id,
        safe_name,
        storage_path,
        content_type,
        size_bytes as i64,
    )
    .await
    .map_err(|_| Status::InternalServerError)?;

    Ok((Status::Created, Json(response)))
}

#[get("/attachments/<id>/file")]
async fn serve_file(
    _user: CurrentUser,
    db: &State<DatabaseConnection>,
    id: &str,
) -> Result<(ContentType, Vec<u8>), Status> {
    let id = Uuid::parse_str(id).map_err(|_| Status::UnprocessableEntity)?;

    let model = attachment_service::find_raw_by_id(db.inner(), id)
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;

    let path = PathBuf::from(upload_dir()).join(&model.storage_path);
    let bytes = tokio::fs::read(&path).await.map_err(|_| Status::NotFound)?;

    let ct = model
        .content_type
        .parse::<ContentType>()
        .unwrap_or(ContentType::Binary);

    Ok((ct, bytes))
}

#[delete("/brews/<_brew_id>/events/<_event_id>/attachments/<id>")]
async fn delete_attachment(
    _user: CurrentUser,
    db: &State<DatabaseConnection>,
    _brew_id: &str,
    _event_id: &str,
    id: &str,
) -> Status {
    let Ok(id) = Uuid::parse_str(id) else {
        return Status::UnprocessableEntity;
    };

    let model = match attachment_service::find_raw_by_id(db.inner(), id).await {
        Ok(Some(m)) => m,
        Ok(None) => return Status::NotFound,
        Err(_) => return Status::InternalServerError,
    };

    let path = PathBuf::from(upload_dir()).join(&model.storage_path);
    let _ = tokio::fs::remove_file(&path).await;

    match attachment_service::delete(db.inner(), id).await {
        Ok(true) => Status::NoContent,
        Ok(false) => Status::NotFound,
        Err(_) => Status::InternalServerError,
    }
}

pub fn routes() -> Vec<Route> {
    routes![upload, serve_file, delete_attachment]
}
