use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{Route, State, get, routes};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::guards::current_user::CurrentUser;
use crate::services::{brew_service, notebook_service, reading_service};
use shared::ReadingsQuery;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotebookUrlResponse {
    pub url: String,
    pub port: u16,
}

#[get("/brews/<id>/notebook")]
pub async fn open_notebook(
    _user: CurrentUser,
    db: &State<DatabaseConnection>,
    registry: &State<notebook_service::NotebookRegistry>,
    id: &str,
) -> Result<Json<NotebookUrlResponse>, Status> {
    let brew_id = Uuid::parse_str(id).map_err(|_| Status::UnprocessableEntity)?;

    let brew = match brew_service::find_by_id(db.inner(), brew_id).await {
        Ok(Some(b)) => b,
        Ok(None) => return Err(Status::NotFound),
        Err(_) => return Err(Status::InternalServerError),
    };

    let query = ReadingsQuery {
        brew_id: Some(brew_id),
        hydrometer_id: None,
        since: None,
        until: None,
        limit: None,
    };
    let readings = reading_service::find_filtered(db.inner(), &query)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to fetch readings for notebook");
            Status::InternalServerError
        })?;

    let notebook_dir = std::env::var("NOTEBOOK_DIR").unwrap_or_else(|_| "./notebooks".to_string());
    let port_base: u16 = std::env::var("MARIMO_PORT_BASE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2718);
    let marimo_host = std::env::var("MARIMO_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

    let port = notebook_service::ensure_notebook_server(
        registry.inner(),
        &brew,
        &readings,
        &notebook_dir,
        port_base,
        &marimo_host,
    )
    .map_err(|e| {
        tracing::error!(brew_id = %brew_id, error = %e, "Failed to start notebook server");
        match e {
            notebook_service::NotebookError::MarimoNotFound => Status::ServiceUnavailable,
            _ => Status::InternalServerError,
        }
    })?;

    let external_host =
        std::env::var("MARIMO_EXTERNAL_HOST").unwrap_or_else(|_| "localhost".to_string());

    let url = format!("http://{}:{}", external_host, port);

    Ok(Json(NotebookUrlResponse { url, port }))
}

pub fn routes() -> Vec<Route> {
    routes![open_notebook]
}
