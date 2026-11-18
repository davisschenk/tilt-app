use rocket::http::{CookieJar, Status};
use rocket::response::Responder;
use rocket::serde::json::Json;
use rocket::{Request, Response, Route, State, get, post, routes};
use sea_orm::DatabaseConnection;
use serde::Deserialize;
use std::path::PathBuf;

use crate::guards::current_user::{CurrentUser, SESSION_COOKIE};
use crate::services::{account_service, user_service};

/// A JSON body the browser saves as a file.
struct Download(serde_json::Value);

impl<'r> Responder<'r, 'static> for Download {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'static> {
        Response::build_from(Json(self.0).respond_to(req)?)
            .raw_header(
                "Content-Disposition",
                "attachment; filename=\"tiltbrew-export.json\"",
            )
            .raw_header("Cache-Control", "no-store")
            .ok()
    }
}

/// Everything the signed-in user owns, as a downloadable JSON file.
#[get("/account/export")]
async fn export(user: CurrentUser, db: &State<DatabaseConnection>) -> Result<Download, Status> {
    let row = user_service::find_by_sub(db.inner(), &user.user_sub)
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;
    account_service::export(db.inner(), &row)
        .await
        .map(Download)
        .map_err(|e| {
            tracing::error!(user_id = %user.user_id, error = %e, "account export failed");
            Status::InternalServerError
        })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteAccount {
    /// Must equal the account's email (case-insensitive); guards against a stray click.
    confirm_email: String,
}

/// Permanently delete the signed-in user and all of their data.
#[post("/account/delete", data = "<input>")]
async fn delete_account(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    cookies: &CookieJar<'_>,
    input: Json<DeleteAccount>,
) -> Status {
    if !input
        .confirm_email
        .trim()
        .eq_ignore_ascii_case(user.email.trim())
    {
        return Status::UnprocessableEntity;
    }
    let row = match user_service::find_by_sub(db.inner(), &user.user_sub).await {
        Ok(Some(r)) => r,
        Ok(None) => return Status::NotFound,
        Err(_) => return Status::InternalServerError,
    };
    let events = match account_service::delete(db.inner(), &row).await {
        Ok(e) => e,
        Err(e) => {
            tracing::error!(user_id = %user.user_id, error = %e, "account deletion failed");
            return Status::InternalServerError;
        }
    };

    // Files go after the commit: an orphaned file is harmless, a row pointing at
    // a missing file is not.
    let base = PathBuf::from(super::attachments::upload_dir());
    for event_id in events {
        let _ = tokio::fs::remove_dir_all(base.join(event_id.to_string())).await;
    }
    cookies.remove_private(SESSION_COOKIE);
    tracing::info!(user_id = %user.user_id, "account deleted");
    Status::NoContent
}

pub fn routes() -> Vec<Route> {
    routes![export, delete_account]
}
