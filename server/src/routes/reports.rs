use rocket::http::{Header, Status};
use rocket::serde::json::Json;
use rocket::{Route, State, delete, get, post, put, routes};
use sea_orm::DatabaseConnection;
use serde_json::Value;
use uuid::Uuid;

use super::attachments::AttachmentFile;
use super::require_brew;
use crate::guards::current_user::CurrentUser;
use crate::services::WriteError;
use crate::services::limits;
use crate::services::report_service::{self, ReportLink, ReportOptions, ReportSummary};

/// A public report response that browsers and proxies must not keep, so a
/// revoked link stops working everywhere.
#[derive(rocket::response::Responder)]
struct NoStore<T> {
    inner: T,
    cache: Header<'static>,
    nosniff: Header<'static>,
}

impl<T> NoStore<T> {
    fn new(inner: T) -> Self {
        NoStore {
            inner,
            cache: Header::new("Cache-Control", "no-store"),
            nosniff: Header::new("X-Content-Type-Options", "nosniff"),
        }
    }
}

fn parse_id(s: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s).map_err(|_| Status::UnprocessableEntity)
}

async fn publish(
    user: CurrentUser,
    db: &DatabaseConnection,
    brew_id: &str,
    options: &ReportOptions,
    replace: Option<&str>,
) -> Result<Json<ReportLink>, Status> {
    let brew_id = parse_id(brew_id)?;
    let replace = replace.map(parse_id).transpose()?;
    if options.event_ids.len() > limits::MAX_REPORT_EVENTS {
        return Err(Status::UnprocessableEntity);
    }
    let brew = require_brew(db, user.user_id, brew_id).await?;
    report_service::publish(db, user.user_id, brew, options, replace)
        .await
        .map(Json)
        .map_err(|e| match e {
            WriteError::NotFound => Status::NotFound,
            WriteError::BadReference => Status::UnprocessableEntity,
            WriteError::LimitReached(_) => Status::Forbidden,
            WriteError::Db(_) => Status::InternalServerError,
        })
}

#[post("/brews/<brew_id>/reports", data = "<options>")]
async fn create_report(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    brew_id: &str,
    options: Json<ReportOptions>,
) -> Result<Json<ReportLink>, Status> {
    publish(user, db.inner(), brew_id, &options, None).await
}

#[put("/brews/<brew_id>/reports/<report_id>", data = "<options>")]
async fn refresh_report(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    brew_id: &str,
    report_id: &str,
    options: Json<ReportOptions>,
) -> Result<Json<ReportLink>, Status> {
    publish(user, db.inner(), brew_id, &options, Some(report_id)).await
}

#[get("/brews/<brew_id>/reports")]
async fn list_reports(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    brew_id: &str,
) -> Result<Json<Vec<ReportSummary>>, Status> {
    let brew_id = parse_id(brew_id)?;
    require_brew(db.inner(), user.user_id, brew_id).await?;
    report_service::list(db.inner(), user.user_id, brew_id)
        .await
        .map(Json)
        .map_err(|_| Status::InternalServerError)
}

#[delete("/brews/<brew_id>/reports/<id>")]
async fn revoke(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    brew_id: &str,
    id: &str,
) -> Result<Status, Status> {
    let brew_id = parse_id(brew_id)?;
    let id = parse_id(id)?;
    require_brew(db.inner(), user.user_id, brew_id).await?;
    match report_service::revoke(db.inner(), user.user_id, brew_id, id).await {
        Ok(true) => Ok(Status::NoContent),
        Ok(false) => Err(Status::NotFound),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[get("/reports/<token>")]
async fn public_report(
    db: &State<DatabaseConnection>,
    token: &str,
) -> Result<NoStore<Json<Value>>, Status> {
    report_service::find_by_token(db.inner(), token)
        .await
        .map_err(|_| Status::InternalServerError)?
        .map(|r| NoStore::new(Json(r.payload)))
        .ok_or(Status::NotFound)
}

#[get("/reports/<token>/photos/<id>")]
async fn public_photo(
    db: &State<DatabaseConnection>,
    token: &str,
    id: &str,
) -> Result<AttachmentFile, Status> {
    let id = parse_id(id)?;
    let attachment = report_service::find_public_photo(db.inner(), token, id)
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;
    AttachmentFile::load(attachment, AttachmentFile::NO_STORE).await
}

pub fn routes() -> Vec<Route> {
    routes![
        create_report,
        refresh_report,
        list_reports,
        revoke,
        public_report,
        public_photo
    ]
}
