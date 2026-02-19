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
        let elapsed_ms = start.0.elapsed().as_millis();
        let method = req.method();
        let uri = req.uri();
        let status = res.status().code;
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
