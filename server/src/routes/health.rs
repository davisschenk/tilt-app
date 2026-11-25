//! Health endpoints for container orchestration.
//!
//! - `GET /health` is liveness: the process is up and serving requests. It never
//!   touches the database, so a database outage does not get the server restarted.
//! - `GET /health/ready` is readiness: the server can reach Postgres. Compose's
//!   healthcheck uses this one, so the container only reports healthy when it can
//!   actually serve requests.
//!
//! Neither takes an auth or rate-limit guard; the rate-limit fairing also skips
//! these paths so frequent probes never use up a client's quota.

use std::time::Duration;

use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{State, get, routes};
use sea_orm::DatabaseConnection;
use serde_json::{Value, json};

/// How long the readiness probe waits for the database before giving up. Kept
/// well under the compose healthcheck timeout (5s).
const DB_PING_TIMEOUT: Duration = Duration::from_secs(2);

pub fn routes() -> Vec<rocket::Route> {
    routes![health, ready]
}

#[get("/health")]
fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

#[get("/health/ready")]
async fn ready(db: &State<DatabaseConnection>) -> (Status, Json<Value>) {
    match rocket::tokio::time::timeout(DB_PING_TIMEOUT, db.ping()).await {
        Ok(Ok(())) => {
            return (
                Status::Ok,
                Json(json!({ "status": "ok", "database": "ok" })),
            );
        }
        Ok(Err(e)) => tracing::warn!("readiness: database ping failed: {e}"),
        Err(_) => tracing::warn!("readiness: database ping timed out after {DB_PING_TIMEOUT:?}"),
    }
    (
        Status::ServiceUnavailable,
        Json(json!({ "status": "unavailable", "database": "error" })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::local::asynchronous::Client;
    use sea_orm::{ConnectOptions, Database};

    async fn client(db: DatabaseConnection) -> Client {
        let rocket = rocket::build().manage(db).mount("/api/v1", routes());
        Client::tracked(rocket).await.expect("valid rocket")
    }

    /// A pool pointed at a port that refuses connections. `connect_lazy` defers the
    /// first connection, so building it succeeds and the ping is what fails.
    async fn unreachable_db() -> DatabaseConnection {
        let mut opts = ConnectOptions::new("postgres://u:p@127.0.0.1:1/none");
        opts.connect_lazy(true)
            .connect_timeout(Duration::from_millis(500))
            .acquire_timeout(Duration::from_millis(500))
            .sqlx_logging(false);
        Database::connect(opts).await.expect("lazy pool builds")
    }

    async fn get(client: &Client, path: &str) -> (Status, Value) {
        let res = client.get(path).dispatch().await;
        let status = res.status();
        let body = res.into_json::<Value>().await.unwrap();
        (status, body)
    }

    #[rocket::async_test]
    async fn liveness_does_not_need_the_database() {
        let client = client(unreachable_db().await).await;
        let (status, body) = get(&client, "/api/v1/health").await;
        assert_eq!(status, Status::Ok);
        assert_eq!(body, json!({ "status": "ok" }));
    }

    #[rocket::async_test]
    async fn readiness_is_503_when_database_is_unreachable() {
        let client = client(unreachable_db().await).await;
        let started = std::time::Instant::now();
        let (status, body) = get(&client, "/api/v1/health/ready").await;
        assert_eq!(status, Status::ServiceUnavailable);
        assert_eq!(
            body,
            json!({ "status": "unavailable", "database": "error" }),
            "error details must not leak into the body"
        );
        assert!(
            started.elapsed() < DB_PING_TIMEOUT + Duration::from_secs(1),
            "probe must answer promptly"
        );
    }

    #[rocket::async_test]
    async fn readiness_is_ok_when_database_answers() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must point at Postgres");
        let client = client(Database::connect(&url).await.expect("connect to Postgres")).await;
        let (status, body) = get(&client, "/api/v1/health/ready").await;
        assert_eq!(status, Status::Ok);
        assert_eq!(body, json!({ "status": "ok", "database": "ok" }));
    }
}
