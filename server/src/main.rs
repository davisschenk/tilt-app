mod fairings;
mod guards;
mod oidc;
mod routes;
mod services;

// Re-export library modules so the binary's existing `crate::auth_mode::...`
// and `crate::models::...` paths keep working.
pub use server::{auth_mode, models};

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
    NamedFile::open(PathBuf::from(&web_dist).join("index.html"))
        .await
        .ok()
}

#[catch(404)]
fn not_found() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "error": "not found" }))
}

#[catch(403)]
fn forbidden() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "error": "forbidden" }))
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
    let frontend_url = std::env::var("FRONTEND_URL").unwrap_or_default();
    let allowed_origins = if frontend_url.is_empty() {
        rocket_cors::AllowedOrigins::all()
    } else {
        let port = std::env::var("ROCKET_PORT").unwrap_or_else(|_| "8000".to_string());
        let mut origins: Vec<String> = frontend_url
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        origins.push(format!("http://localhost:{port}"));
        origins.push(format!("http://127.0.0.1:{port}"));
        origins.dedup();
        let origin_refs: Vec<&str> = origins.iter().map(String::as_str).collect();
        rocket_cors::AllowedOrigins::some_exact(&origin_refs)
    };

    rocket_cors::CorsOptions {
        allowed_origins,
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
        allow_credentials: true,
        ..Default::default()
    }
    .to_cors()
    .expect("CORS configuration failed")
}

#[rocket::launch]
async fn rocket() -> Rocket<Build> {
    dotenvy::dotenv().ok();

    let log_filter = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "info,rocket::response::debug=off".to_string());
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(log_filter))
        .init();

    tracing::info!("Starting Tilt Hydrometer Platform server");

    let db = setup_db().await;

    tracing::info!("Running database migrations");
    migration::Migrator::up(&db, None)
        .await
        .expect("Failed to run database migrations");
    tracing::info!("Migrations complete");

    let cors = setup_cors();

    let auth_mode = match auth_mode::AuthMode::from_env() {
        Ok(m) => m,
        Err(e) => {
            tracing::error!("Invalid AUTH_MODE: {e}");
            std::process::exit(1);
        }
    };

    let oidc_state = match auth_mode {
        auth_mode::AuthMode::Disabled => {
            tracing::warn!("auth: disabled (dev user injected — do NOT use in production)");
            None
        }
        auth_mode::AuthMode::Oidc => {
            let oidc_issuer = std::env::var("AUTHENTIK_ISSUER_URL").unwrap_or_default();
            let oidc_client_id = std::env::var("AUTHENTIK_CLIENT_ID").unwrap_or_default();
            let oidc_client_secret = std::env::var("AUTHENTIK_CLIENT_SECRET").unwrap_or_default();
            let oidc_redirect_url = std::env::var("AUTHENTIK_REDIRECT_URL")
                .unwrap_or_else(|_| "http://localhost:8000/api/v1/auth/callback".to_string());

            if oidc_issuer.is_empty() || oidc_client_id.is_empty() || oidc_client_secret.is_empty()
            {
                tracing::error!(
                    "AUTH_MODE=oidc requires AUTHENTIK_ISSUER_URL, AUTHENTIK_CLIENT_ID, and AUTHENTIK_CLIENT_SECRET to be set"
                );
                std::process::exit(1);
            }

            match oidc::OidcState::discover(
                &oidc_issuer,
                &oidc_client_id,
                &oidc_client_secret,
                &oidc_redirect_url,
            )
            .await
            {
                Ok(state) => {
                    tracing::info!("auth: oidc {oidc_issuer}");
                    Some(state)
                }
                Err(e) => {
                    tracing::error!("OIDC discovery failed: {e}");
                    std::process::exit(1);
                }
            }
        }
    };

    let web_dist = std::env::var("WEB_DIST_DIR")
        .unwrap_or_else(|_| "web/dist".to_string())
        .trim()
        .to_string();

    let http_client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("Failed to build HTTP client");

    let mut rocket = rocket::build()
        .manage(db)
        .manage(http_client)
        .attach(cors)
        .attach(fairings::rate_limit::RateLimit::new())
        .attach(fairings::request_logger::RequestLogger)
        .attach(fairings::security_headers::SecurityHeaders)
        .attach(fairings::session_cleanup::SessionCleanup)
        .mount("/api/v1", routes![health])
        .mount("/", routes![preflight])
        .mount("/api/v1", routes::hydrometers::routes())
        .mount("/api/v1", routes::brews::routes())
        .mount("/api/v1", routes::readings::routes())
        .mount("/api/v1", routes::alert_targets::routes())
        .mount("/api/v1", routes::alert_rules::routes())
        .mount("/api/v1", routes::brew_events::routes())
        .mount("/api/v1", routes::attachments::routes())
        .mount("/api/v1", routes::analytics::routes())
        .mount("/api/v1", routes::ota::routes())
        .mount("/api/v1", routes::tosna::routes())
        .mount("/api/v1", routes::yeast::routes())
        .mount("/", FileServer::from(PathBuf::from(&web_dist)))
        .mount("/", routes![spa_fallback])
        .register(
            "/",
            catchers![not_found, forbidden, unprocessable_entity, internal_error],
        );

    rocket = rocket
        .manage(oidc_state)
        .mount("/api/v1", routes::auth::routes())
        .mount("/api/v1", routes::api_keys::routes());

    rocket
}
