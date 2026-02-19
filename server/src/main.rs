mod guards;
mod models;
mod oidc;
mod routes;
mod services;

use rocket::fs::{FileServer, NamedFile};
use rocket::serde::json::Json;
use rocket::{Build, Rocket, catch, catchers, get, options, routes};
use sea_orm::{Database, DatabaseConnection};
use sea_orm_migration::MigratorTrait;
use std::path::PathBuf;

#[get("/health")]
fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

#[options("/<_path..>")]
fn preflight(_path: PathBuf) -> rocket::http::Status {
    rocket::http::Status::NoContent
}

#[get("/<_path..>", rank = 100)]
async fn spa_fallback(_path: PathBuf) -> Option<NamedFile> {
    let web_dist = std::env::var("WEB_DIST_DIR")
        .unwrap_or_else(|_| "web/dist".to_string())
        .trim()
        .to_string();
    NamedFile::open(PathBuf::from(&web_dist).join("index.html")).await.ok()
}

#[catch(404)]
fn not_found() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "error": "not found" }))
}

#[catch(422)]
fn unprocessable_entity() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "error": "unprocessable entity" }))
}

#[catch(500)]
fn internal_error() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "error": "internal server error" }))
}

async fn setup_db() -> DatabaseConnection {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    Database::connect(&database_url)
        .await
        .expect("Failed to connect to database")
}

fn setup_cors() -> rocket_cors::Cors {
    rocket_cors::CorsOptions {
        allowed_origins: rocket_cors::AllowedOrigins::all(),
        allowed_methods: vec![
            rocket::http::Method::Get,
            rocket::http::Method::Post,
            rocket::http::Method::Put,
            rocket::http::Method::Delete,
            rocket::http::Method::Options,
        ]
        .into_iter()
        .map(From::from)
        .collect(),
        allowed_headers: rocket_cors::AllowedHeaders::all(),
        allow_credentials: false,
        ..Default::default()
    }
    .to_cors()
    .expect("CORS configuration failed")
}

#[rocket::launch]
async fn rocket() -> Rocket<Build> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("Starting Tilt Hydrometer Platform server");

    let db = setup_db().await;

    tracing::info!("Running database migrations");
    migration::Migrator::up(&db, None)
        .await
        .expect("Failed to run database migrations");
    tracing::info!("Migrations complete");

    let cors = setup_cors();

    let oidc_issuer = std::env::var("AUTHENTIK_ISSUER_URL").unwrap_or_default();
    let oidc_client_id = std::env::var("AUTHENTIK_CLIENT_ID").unwrap_or_default();
    let oidc_client_secret = std::env::var("AUTHENTIK_CLIENT_SECRET").unwrap_or_default();
    let oidc_redirect_url = std::env::var("AUTHENTIK_REDIRECT_URL")
        .unwrap_or_else(|_| "http://localhost:8000/api/v1/auth/callback".to_string());

    let oidc_state = if !oidc_issuer.is_empty() {
        match oidc::OidcState::discover(
            &oidc_issuer,
            &oidc_client_id,
            &oidc_client_secret,
            &oidc_redirect_url,
        )
        .await
        {
            Ok(state) => {
                tracing::info!("OIDC discovery successful");
                Some(state)
            }
            Err(e) => {
                tracing::warn!("OIDC discovery failed: {e} — auth routes will be unavailable");
                None
            }
        }
    } else {
        tracing::warn!("AUTHENTIK_ISSUER_URL not set — auth routes will be unavailable");
        None
    };

    let web_dist = std::env::var("WEB_DIST_DIR")
        .unwrap_or_else(|_| "web/dist".to_string())
        .trim()
        .to_string();

    let mut rocket = rocket::build()
        .manage(db)
        .attach(cors)
        .mount("/api/v1", routes![health])
        .mount("/", routes![preflight])
        .mount("/api/v1", routes::hydrometers::routes())
        .mount("/api/v1", routes::brews::routes())
        .mount("/api/v1", routes::readings::routes())
        .mount("/", FileServer::from(PathBuf::from(&web_dist)))
        .mount("/", routes![spa_fallback])
        .register(
            "/",
            catchers![not_found, unprocessable_entity, internal_error],
        );

    if let Some(oidc) = oidc_state {
        rocket = rocket
            .manage(oidc)
            .mount("/api/v1", routes::auth::routes());
    }

    rocket
}
