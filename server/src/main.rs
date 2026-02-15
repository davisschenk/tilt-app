mod models;
mod routes;
mod services;

use rocket::serde::json::Json;
use rocket::{Build, Rocket, catch, catchers, get, routes};
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
        allow_credentials: true,
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
    let cors = setup_cors();

    rocket::build()
        .manage(db)
        .attach(cors)
        .mount("/api/v1", routes![health])
        .mount("/api/v1", routes::hydrometers::routes())
        .mount("/api/v1", routes::brews::routes())
        .mount("/api/v1", routes::readings::routes())
        .register(
            "/",
            catchers![not_found, unprocessable_entity, internal_error],
        )
}
