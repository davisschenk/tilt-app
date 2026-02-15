mod models;
mod routes;

use rocket::serde::json::Json;
use rocket::{catchers, catch, get, routes, Build, Rocket};
use sea_orm::{Database, DatabaseConnection};

#[get("/health")]
fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
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
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    Database::connect(&database_url)
        .await
        .expect("Failed to connect to database")
}

fn setup_cors() -> rocket_cors::Cors {
    rocket_cors::CorsOptions::default()
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
    let cors = setup_cors();

    rocket::build()
        .manage(db)
        .attach(cors)
        .mount("/api/v1", routes![health])
        .register("/", catchers![not_found, unprocessable_entity, internal_error])
}
