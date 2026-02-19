use std::{
    collections::HashMap,
    net::IpAddr,
    num::NonZeroU32,
    sync::Arc,
    time::Duration,
};

use governor::{
    DefaultDirectRateLimiter, Quota, RateLimiter,
    clock::DefaultClock,
    state::keyed::DefaultKeyedStateStore,
};
use rocket::{
    Data, Request,
    fairing::{Fairing, Info, Kind},
    http::Status,
    outcome::Outcome,
};
use tokio::sync::Mutex;

type KeyedLimiter = RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>;

pub struct RateLimit {
    /// Limiter for auth endpoints: 20 req/min per IP
    auth_limiter: Arc<KeyedLimiter>,
    /// Limiter for general API endpoints: 300 req/min per IP
    api_limiter: Arc<KeyedLimiter>,
}

impl RateLimit {
    pub fn new() -> Self {
        let auth_quota = Quota::per_minute(NonZeroU32::new(20).unwrap());
        let api_quota = Quota::per_minute(NonZeroU32::new(300).unwrap());

        Self {
            auth_limiter: Arc::new(RateLimiter::keyed(auth_quota)),
            api_limiter: Arc::new(RateLimiter::keyed(api_quota)),
        }
    }
}

fn client_ip(req: &Request<'_>) -> IpAddr {
    req.client_ip()
        .unwrap_or(IpAddr::from([127, 0, 0, 1]))
}

fn is_auth_path(path: &str) -> bool {
    path.starts_with("/api/v1/auth/")
}

fn is_api_path(path: &str) -> bool {
    path.starts_with("/api/v1/")
}

#[rocket::async_trait]
impl Fairing for RateLimit {
    fn info(&self) -> Info {
        Info {
            name: "Rate Limiter",
            kind: Kind::Request,
        }
    }

    async fn on_request(&self, req: &mut Request<'_>, _data: &mut Data<'_>) {
        let path = req.uri().path().to_string();
        let ip = client_ip(req);

        if is_auth_path(&path) {
            if self.auth_limiter.check_key(&ip).is_err() {
                req.local_cache(|| RateLimitExceeded(true));
            }
        } else if is_api_path(&path) {
            if self.api_limiter.check_key(&ip).is_err() {
                req.local_cache(|| RateLimitExceeded(true));
            }
        }
    }
}

/// Marker stored in request-local cache when rate limit is exceeded.
#[derive(Clone, Copy)]
pub struct RateLimitExceeded(pub bool);

/// Request guard that returns 429 if rate limit was exceeded.
pub struct RateLimitGuard;

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for RateLimitGuard {
    type Error = ();

    async fn from_request(
        req: &'r Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        if req
            .local_cache(|| RateLimitExceeded(false))
            .0
        {
            Outcome::Error((Status::TooManyRequests, ()))
        } else {
            Outcome::Success(RateLimitGuard)
        }
    }
}
