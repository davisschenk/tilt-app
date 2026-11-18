use rocket::form::Form;
use rocket::fs::TempFile;
use rocket::http::{ContentType, Status};
use rocket::serde::json::Json;
use rocket::{Route, State, delete, get, post, routes};
use sea_orm::DatabaseConnection;
use std::path::PathBuf;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

use shared::EventAttachmentResponse;

use crate::guards::current_user::CurrentUser;
use crate::routes::require_brew;
use crate::services::{attachment_service, brew_event_service};

const MAX_FILE_BYTES: u64 = 10 * 1024 * 1024; // 10 MB
/// Total attachment storage one user may hold.
const MAX_USER_BYTES: i64 = 250 * 1024 * 1024;

/// Identify an image by its magic bytes. Only raster formats a browser cannot
/// execute are allowed; SVG (script-capable) is deliberately not one of them.
/// Returns `(mime type, file extension)`.
fn sniff_image(head: &[u8]) -> Option<(&'static str, &'static str)> {
    if head.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(("image/png", "png"))
    } else if head.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(("image/jpeg", "jpg"))
    } else if head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a") {
        Some(("image/gif", "gif"))
    } else if head.len() >= 12 && &head[0..4] == b"RIFF" && &head[8..12] == b"WEBP" {
        Some(("image/webp", "webp"))
    } else {
        None
    }
}

/// An uploaded file served back to its owner. The headers make it inert even
/// if the bytes are not what we think: no sniffing, no scripts, sandboxed.
struct AttachmentFile {
    content_type: ContentType,
    bytes: Vec<u8>,
    filename: String,
}

impl<'r> rocket::response::Responder<'r, 'static> for AttachmentFile {
    fn respond_to(self, req: &'r rocket::Request<'_>) -> rocket::response::Result<'static> {
        rocket::Response::build_from(self.bytes.respond_to(req)?)
            .header(self.content_type)
            .raw_header("X-Content-Type-Options", "nosniff")
            .raw_header(
                "Content-Security-Policy",
                "default-src 'none'; style-src 'unsafe-inline'; sandbox",
            )
            .raw_header(
                "Content-Disposition",
                format!("inline; filename=\"{}\"", self.filename),
            )
            .raw_header("Cache-Control", "private, max-age=3600")
            .ok()
    }
}

pub(crate) fn upload_dir() -> String {
    std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "./uploads".to_string())
}

#[post("/brews/<brew_id>/events/<event_id>/attachments", data = "<upload>")]
async fn upload(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    brew_id: &str,
    event_id: &str,
    upload: Form<TempFile<'_>>,
) -> Result<(Status, Json<EventAttachmentResponse>), Status> {
    let mut file = upload.into_inner();
    let brew_id = Uuid::parse_str(brew_id).map_err(|_| Status::UnprocessableEntity)?;
    let event_id = Uuid::parse_str(event_id).map_err(|_| Status::UnprocessableEntity)?;

    require_brew(db.inner(), user.user_id, brew_id).await?;
    brew_event_service::find_in_brew_raw(db.inner(), brew_id, event_id)
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;

    let size_bytes = file.len();
    if size_bytes > MAX_FILE_BYTES {
        return Err(Status::PayloadTooLarge);
    }

    // Trust the bytes, not the client's Content-Type or filename.
    let mut head = Vec::with_capacity(16);
    file.open()
        .await
        .map_err(|_| Status::InternalServerError)?
        .take(16)
        .read_to_end(&mut head)
        .await
        .map_err(|_| Status::InternalServerError)?;
    let (content_type, ext) = sniff_image(&head).ok_or(Status::UnprocessableEntity)?;
    let content_type = content_type.to_string();

    let used = attachment_service::total_bytes_for_owner(db.inner(), user.user_id)
        .await
        .map_err(|_| Status::InternalServerError)?;
    if used + size_bytes as i64 > MAX_USER_BYTES {
        return Err(Status::InsufficientStorage);
    }

    let original_name = file
        .raw_name()
        .map(|n| n.dangerous_unsafe_unsanitized_raw().as_str().to_string())
        .unwrap_or_else(|| "upload".to_string());

    // Keep the user's name for display, but the extension always matches the
    // detected type, never whatever the client claimed.
    let stem = original_name
        .rsplit_once('.')
        .map_or(original_name.as_str(), |(stem, _)| stem);
    let stem: String = stem
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '_'
            }
        })
        .take(64)
        .collect();
    let stem = if stem.is_empty() {
        "upload".to_string()
    } else {
        stem
    };
    let safe_name = format!("{stem}.{ext}");

    let attachment_id = Uuid::new_v4();
    let filename_on_disk = format!("{}_{}", attachment_id, safe_name);
    let dir = PathBuf::from(upload_dir()).join(event_id.to_string());
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let dest = dir.join(&filename_on_disk);
    file.copy_to(&dest)
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
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    id: &str,
) -> Result<AttachmentFile, Status> {
    let id = Uuid::parse_str(id).map_err(|_| Status::UnprocessableEntity)?;

    let model = attachment_service::find_owned_raw(db.inner(), user.user_id, id)
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;

    let path = PathBuf::from(upload_dir()).join(&model.storage_path);
    let bytes = tokio::fs::read(&path).await.map_err(|_| Status::NotFound)?;

    let content_type = model
        .content_type
        .parse::<ContentType>()
        .unwrap_or(ContentType::Binary);

    Ok(AttachmentFile {
        content_type,
        bytes,
        filename: model.filename,
    })
}

#[delete("/brews/<_brew_id>/events/<event_id>/attachments/<id>")]
async fn delete_attachment(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    _brew_id: &str,
    event_id: &str,
    id: &str,
) -> Status {
    let (Ok(id), Ok(event_id)) = (Uuid::parse_str(id), Uuid::parse_str(event_id)) else {
        return Status::UnprocessableEntity;
    };

    let model = match attachment_service::find_owned_raw(db.inner(), user.user_id, id).await {
        Ok(Some(m)) if m.event_id == event_id => m,
        Ok(_) => return Status::NotFound,
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

#[cfg(test)]
mod tests {
    use super::sniff_image;

    #[test]
    fn accepts_real_raster_images() {
        assert_eq!(
            sniff_image(b"\x89PNG\r\n\x1a\n\0\0"),
            Some(("image/png", "png"))
        );
        assert_eq!(
            sniff_image(&[0xFF, 0xD8, 0xFF, 0xE0, 0]),
            Some(("image/jpeg", "jpg"))
        );
        assert_eq!(sniff_image(b"GIF89a\x01\0"), Some(("image/gif", "gif")));
        assert_eq!(
            sniff_image(b"RIFF\x10\0\0\0WEBPVP8 "),
            Some(("image/webp", "webp"))
        );
    }

    #[test]
    fn rejects_script_capable_and_mislabelled_files() {
        assert_eq!(
            sniff_image(b"<svg xmlns='http://www.w3.org/2000/svg' onload='x()'/>"),
            None
        );
        assert_eq!(sniff_image(b"<?xml version='1.0'?><svg/>"), None);
        assert_eq!(sniff_image(b"<html><script>alert(1)</script>"), None);
        assert_eq!(sniff_image(b"RIFF\x10\0\0\0WAVEfmt "), None);
        assert_eq!(sniff_image(b""), None);
    }
}
