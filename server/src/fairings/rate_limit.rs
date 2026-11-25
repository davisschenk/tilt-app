use std::{net::IpAddr, num::NonZeroU32, sync::Arc};

use governor::{Quota, RateLimiter, clock::DefaultClock, state::keyed::DefaultKeyedStateStore};
use rocket::{
    Data, Request,
    fairing::{Fairing, Info, Kind},
    http::Status,
    outcome::Outcome,
};

type KeyedLimiter = RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>;

const AUTH_PER_MINUTE: NonZeroU32 = NonZeroU32::new(20).unwrap();
const API_PER_MINUTE: NonZeroU32 = NonZeroU32::new(300).unwrap();
const TEST_FIRE_PER_MINUTE: NonZeroU32 = NonZeroU32::new(10).unwrap();

pub struct RateLimit {
    /// Limiter for auth endpoints: 20 req/min per IP
    auth_limiter: Arc<KeyedLimiter>,
    /// Limiter for general API endpoints: 300 req/min per IP
    api_limiter: Arc<KeyedLimiter>,
}

impl RateLimit {
    pub fn new() -> Self {
        Self {
            auth_limiter: Arc::new(RateLimiter::keyed(Quota::per_minute(AUTH_PER_MINUTE))),
            api_limiter: Arc::new(RateLimiter::keyed(Quota::per_minute(API_PER_MINUTE))),
        }
    }
}

fn client_ip(req: &Request<'_>) -> IpAddr {
    req.client_ip().unwrap_or(IpAddr::from([127, 0, 0, 1]))
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
        } else if is_api_path(&path) && self.api_limiter.check_key(&ip).is_err() {
            req.local_cache(|| RateLimitExceeded(true));
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

    async fn from_request(req: &'r Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        if req.local_cache(|| RateLimitExceeded(false)).0 {
            Outcome::Error((Status::TooManyRequests, ()))
        } else {
            Outcome::Success(RateLimitGuard)
        }
    }
}

/// Per-user limiter for endpoints that make the server send a request on the
/// user's behalf (webhook test-fire), so one account cannot use them as a cannon.
pub struct TestFireLimiter(
    RateLimiter<uuid::Uuid, DefaultKeyedStateStore<uuid::Uuid>, DefaultClock>,
);

impl TestFireLimiter {
    pub fn new() -> Self {
        Self(RateLimiter::keyed(Quota::per_minute(TEST_FIRE_PER_MINUTE)))
    }

    /// True if `user` may fire another test right now.
    pub fn allow(&self, user: uuid::Uuid) -> bool {
        self.0.check_key(&user).is_ok()
    }
}

/// Per-user request limiter for the authenticated API. The IP limiter above only
/// guards a few routes and keys on a shared address; this one follows the
/// account, whichever network or API key the request comes from.
pub struct ApiUserLimiter(
    RateLimiter<uuid::Uuid, DefaultKeyedStateStore<uuid::Uuid>, DefaultClock>,
);

impl ApiUserLimiter {
    /// Requests per minute each account may make. The web UI polls a handful of
    /// endpoints every 30s, so the default leaves a wide margin.
    pub const DEFAULT_PER_MINUTE: NonZeroU32 = NonZeroU32::new(600).unwrap();

    pub fn new(per_minute: NonZeroU32) -> Self {
        Self(RateLimiter::keyed(Quota::per_minute(per_minute)))
    }

    /// True if `user` may make another request right now. If no limiter is
    /// installed (some tests), requests are allowed.
    pub fn allow(req: &Request<'_>, user: uuid::Uuid) -> bool {
        req.rocket()
            .state::<ApiUserLimiter>()
            .is_none_or(|l| l.0.check_key(&user).is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::{get, local::asynchronous::Client, routes};
    use std::net::IpAddr;

    #[get("/ip")]
    fn ip(ip: IpAddr) -> String {
        ip.to_string()
    }

    async fn seen_ip(figment: rocket::figment::Figment) -> String {
        let rocket = rocket::custom(figment).mount("/", routes![ip]);
        let client = Client::tracked(rocket).await.unwrap();
        client
            .get("/ip")
            .remote("9.9.9.9:4000".parse().unwrap())
            .header(rocket::http::Header::new("X-Real-IP", "1.2.3.4"))
            .dispatch()
            .await
            .into_string()
            .await
            .unwrap()
    }

    #[rocket::async_test]
    async fn spoofed_client_ip_header_is_ignored_by_default() {
        // `rocket::build()` reads Rocket.toml from the crate root, which disables ip_header.
        let seen = seen_ip(rocket::Config::figment()).await;
        assert_eq!(
            seen, "9.9.9.9",
            "X-Real-IP must not be trusted unless configured"
        );
    }

    #[rocket::async_test]
    async fn named_proxy_header_is_trusted_when_configured() {
        let figment = rocket::Config::figment().merge(("ip_header", "X-Real-IP"));
        assert_eq!(seen_ip(figment).await, "1.2.3.4");
    }

    #[test]
    fn per_user_limit_is_per_user() {
        let l = ApiUserLimiter::new(NonZeroU32::new(2).unwrap());
        let (a, b) = (uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
        assert!(l.0.check_key(&a).is_ok());
        assert!(l.0.check_key(&a).is_ok());
        assert!(
            l.0.check_key(&a).is_err(),
            "third request in a minute is refused"
        );
        assert!(l.0.check_key(&b).is_ok(), "another user is unaffected");
    }
}
