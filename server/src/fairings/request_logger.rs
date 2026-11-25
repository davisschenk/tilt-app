use rocket::{
    Data, Request, Response,
    fairing::{Fairing, Info, Kind},
};
use std::time::Instant;

pub struct RequestLogger;

#[rocket::async_trait]
impl Fairing for RequestLogger {
    fn info(&self) -> Info {
        Info {
            name: "Request Logger",
            kind: Kind::Request | Kind::Response,
        }
    }

    async fn on_request(&self, req: &mut Request<'_>, _data: &mut Data<'_>) {
        req.local_cache(|| RequestStart(Instant::now()));
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        let start = req.local_cache(|| RequestStart(Instant::now()));
        let elapsed = start.0.elapsed();
        let elapsed_ms = elapsed.as_millis();
        let method = req.method();
        let uri = req.uri();
        let status = res.status().code;

        // Label by route template, never the raw path, so cardinality stays bounded.
        // 404s may have tried (and forwarded past) a route, so they are all "unmatched".
        let route = match req.route() {
            Some(r) if status != 404 => r.uri.path(),
            _ => "unmatched",
        };
        crate::metrics::observe_http(method.as_str(), route, status, elapsed.as_secs_f64());
        let ip = req
            .client_ip()
            .map(|ip| ip.to_string())
            .unwrap_or_else(|| "-".to_string());

        tracing::info!(
            method = %method,
            uri = %uri,
            status = status,
            elapsed_ms = elapsed_ms,
            ip = %ip,
            "request"
        );
    }
}

#[derive(Clone, Copy)]
struct RequestStart(Instant);
