//! Prometheus metrics, served at `GET /metrics` behind a bearer token.
//!
//! The endpoint only exists when `METRICS_TOKEN` is set; otherwise it answers
//! 404 so a default deployment exposes nothing. Labels are kept to bounded sets
//! (method, route template, status class, fixed reasons): never users or devices.

use std::sync::LazyLock;

use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGaugeVec, Opts, Registry,
    TextEncoder,
};
use rocket::http::{ContentType, Status};
use rocket::request::{FromRequest, Outcome, Request};
use rocket::{Route, get, routes};
use sea_orm::DatabaseConnection;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

struct Metrics {
    registry: Registry,
    http_requests: IntCounterVec,
    http_duration: HistogramVec,
    readings_accepted: IntCounter,
    reading_batches_rejected: IntCounterVec,
    webhook_deliveries: IntCounterVec,
    db_pool_connections: IntGaugeVec,
}

static METRICS: LazyLock<Metrics> = LazyLock::new(|| {
    let registry = Registry::new();
    let http_requests = IntCounterVec::new(
        Opts::new("tilt_http_requests_total", "HTTP requests handled"),
        &["method", "route", "status"],
    )
    .unwrap();
    let http_duration = HistogramVec::new(
        HistogramOpts::new(
            "tilt_http_request_duration_seconds",
            "HTTP request latency in seconds",
        ),
        &["method", "route", "status"],
    )
    .unwrap();
    let readings_accepted = IntCounter::new(
        "tilt_readings_accepted_total",
        "Hydrometer readings stored via POST /readings",
    )
    .unwrap();
    let reading_batches_rejected = IntCounterVec::new(
        Opts::new(
            "tilt_reading_batches_rejected_total",
            "POST /readings requests rejected, by reason",
        ),
        &["reason"],
    )
    .unwrap();
    let webhook_deliveries = IntCounterVec::new(
        Opts::new(
            "tilt_webhook_deliveries_total",
            "Outgoing webhook deliveries",
        ),
        &["kind", "outcome"],
    )
    .unwrap();
    let db_pool_connections = IntGaugeVec::new(
        Opts::new(
            "tilt_db_pool_connections",
            "Database pool connections by state (sampled at scrape time)",
        ),
        &["state"],
    )
    .unwrap();

    registry.register(Box::new(http_requests.clone())).unwrap();
    registry.register(Box::new(http_duration.clone())).unwrap();
    registry
        .register(Box::new(readings_accepted.clone()))
        .unwrap();
    registry
        .register(Box::new(reading_batches_rejected.clone()))
        .unwrap();
    registry
        .register(Box::new(webhook_deliveries.clone()))
        .unwrap();
    registry
        .register(Box::new(db_pool_connections.clone()))
        .unwrap();

    Metrics {
        registry,
        http_requests,
        http_duration,
        readings_accepted,
        reading_batches_rejected,
        webhook_deliveries,
        db_pool_connections,
    }
});

/// Record one finished HTTP request. `route` must be a route template (or
/// "unmatched"), never a raw path.
pub fn observe_http(method: &str, route: &str, status: u16, seconds: f64) {
    let class = match status {
        100..=199 => "1xx",
        200..=299 => "2xx",
        300..=399 => "3xx",
        400..=499 => "4xx",
        _ => "5xx",
    };
    let labels = [method, route, class];
    let m = &*METRICS;
    m.http_requests.with_label_values(&labels).inc();
    m.http_duration.with_label_values(&labels).observe(seconds);
}

pub fn readings_accepted(n: u64) {
    METRICS.readings_accepted.inc_by(n);
}

/// Why a reading batch was refused.
#[derive(Clone, Copy)]
pub enum RejectReason {
    RateLimited,
    Cap,
    Invalid,
}

pub fn reading_batch_rejected(reason: RejectReason) {
    let label = match reason {
        RejectReason::RateLimited => "rate_limited",
        RejectReason::Cap => "cap",
        RejectReason::Invalid => "invalid",
    };
    METRICS
        .reading_batches_rejected
        .with_label_values(&[label])
        .inc();
}

/// `kind` is one of a fixed set: "alert", "nutrient", "test".
pub fn webhook_delivery(kind: &'static str, ok: bool) {
    let outcome = if ok { "success" } else { "failure" };
    METRICS
        .webhook_deliveries
        .with_label_values(&[kind, outcome])
        .inc();
}

/// The configured scrape token. `None` (or empty) disables the endpoint.
pub struct MetricsToken(Option<String>);

impl MetricsToken {
    pub fn new(token: Option<String>) -> Self {
        Self(
            token
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty()),
        )
    }

    pub fn from_env() -> Self {
        Self::new(std::env::var("METRICS_TOKEN").ok())
    }

    fn matches(&self, presented: Option<&str>) -> bool {
        let (Some(expected), Some(presented)) = (self.0.as_deref(), presented) else {
            return false;
        };
        // Hash both sides so the comparison is fixed-length and constant-time.
        let a = Sha256::digest(expected.as_bytes());
        let b = Sha256::digest(presented.as_bytes());
        a.ct_eq(&b).into()
    }
}

/// The raw `Authorization: Bearer <token>` value, plus the app's managed state.
/// State is looked up at runtime rather than via `&State` so a missing token (or
/// database, in tests) is a 404 instead of a launch failure.
struct Scrape<'r> {
    bearer: Option<&'r str>,
    rocket: &'r rocket::Rocket<rocket::Orbit>,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for Scrape<'r> {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        Outcome::Success(Scrape {
            bearer: req
                .headers()
                .get_one("Authorization")
                .and_then(|v| v.strip_prefix("Bearer ")),
            rocket: req.rocket(),
        })
    }
}

fn sample_db_pool(db: &DatabaseConnection) {
    if let DatabaseConnection::SqlxPostgresPoolConnection(_) = db {
        let pool = db.get_postgres_connection_pool();
        let idle = pool.num_idle() as i64;
        let in_use = (pool.size() as i64 - idle).max(0);
        let g = &METRICS.db_pool_connections;
        g.with_label_values(&["idle"]).set(idle);
        g.with_label_values(&["in_use"]).set(in_use);
    }
}

#[get("/metrics")]
fn metrics(scrape: Scrape<'_>) -> Result<(ContentType, Vec<u8>), Status> {
    let token = scrape
        .rocket
        .state::<MetricsToken>()
        .filter(|t| t.0.is_some())
        .ok_or(Status::NotFound)?;
    if !token.matches(scrape.bearer) {
        return Err(Status::Unauthorized);
    }
    if let Some(db) = scrape.rocket.state::<DatabaseConnection>() {
        sample_db_pool(db);
    }

    let mut buf = Vec::new();
    TextEncoder::new()
        .encode(&METRICS.registry.gather(), &mut buf)
        .map_err(|e| {
            tracing::error!(error = %e, "failed to encode metrics");
            Status::InternalServerError
        })?;
    let content_type = ContentType::new("text", "plain").with_params(("version", "0.0.4"));
    Ok((content_type, buf))
}

pub fn routes() -> Vec<Route> {
    routes![metrics]
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::http::Header;
    use rocket::local::asynchronous::Client;

    #[get("/api/v1/things/<_id>")]
    fn thing(_id: u32) -> &'static str {
        "ok"
    }

    async fn client(token: Option<&str>) -> Client {
        let mut rocket = rocket::build()
            .attach(crate::fairings::request_logger::RequestLogger)
            .mount("/", routes())
            .mount("/", routes![thing])
            // Disconnected: readiness answers 503 without a real database.
            .manage(DatabaseConnection::Disconnected)
            .mount("/api/v1", crate::routes::health::routes());
        if let Some(t) = token {
            rocket = rocket.manage(MetricsToken::new(Some(t.to_string())));
        }
        Client::tracked(rocket).await.unwrap()
    }

    #[rocket::async_test]
    async fn not_found_without_token_configured() {
        let c = client(None).await;
        let res = c
            .get("/metrics")
            .header(Header::new("Authorization", "Bearer anything"))
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::NotFound);

        let c = rocket::build()
            .manage(MetricsToken::new(Some("  ".into())))
            .mount("/", routes());
        let c = Client::tracked(c).await.unwrap();
        assert_eq!(
            c.get("/metrics").dispatch().await.status(),
            Status::NotFound,
            "blank token counts as unset"
        );
    }

    #[rocket::async_test]
    async fn unauthorized_with_wrong_or_missing_token() {
        let c = client(Some("s3cret")).await;
        assert_eq!(
            c.get("/metrics").dispatch().await.status(),
            Status::Unauthorized
        );
        let res = c
            .get("/metrics")
            .header(Header::new("Authorization", "Bearer wrong"))
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Unauthorized);
    }

    #[rocket::async_test]
    async fn exposes_metrics_with_right_token() {
        let c = client(Some("s3cret")).await;
        assert_eq!(
            c.get("/api/v1/things/42").dispatch().await.status(),
            Status::Ok
        );
        assert_eq!(
            c.get("/api/v1/health").dispatch().await.status(),
            Status::Ok
        );
        assert_eq!(
            c.get("/api/v1/health/ready").dispatch().await.status(),
            Status::ServiceUnavailable
        );
        assert_eq!(
            c.get("/no/such/path").dispatch().await.status(),
            Status::NotFound
        );
        readings_accepted(3);
        reading_batch_rejected(RejectReason::Cap);
        webhook_delivery("test", true);

        let res = c
            .get("/metrics")
            .header(Header::new("Authorization", "Bearer s3cret"))
            .dispatch()
            .await;
        assert_eq!(res.status(), Status::Ok);
        assert_eq!(res.content_type().unwrap().media_type().sub(), "plain");
        let body = res.into_string().await.unwrap();
        for name in [
            "tilt_http_requests_total",
            "tilt_http_request_duration_seconds_bucket",
            "tilt_readings_accepted_total",
            "tilt_reading_batches_rejected_total",
            "tilt_webhook_deliveries_total",
        ] {
            assert!(body.contains(name), "missing {name} in:\n{body}");
        }
        assert!(
            body.contains(r#"route="/api/v1/things/<_id>""#),
            "labels use the route template:\n{body}"
        );
        assert!(
            !body.contains("/api/v1/things/42"),
            "raw paths never become labels"
        );
        assert!(body.contains(r#"route="unmatched""#), "404s are unmatched");
        assert!(
            body.contains(r#"method="GET",route="/api/v1/health/ready",status="5xx""#),
            "health probes are labelled by route template:\n{body}"
        );
        assert!(body.contains(r#"route="/api/v1/health""#));
        assert!(body.contains(r#"status="2xx""#), "status is bucketed");
    }
}
