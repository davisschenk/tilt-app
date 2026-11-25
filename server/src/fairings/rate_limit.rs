//! Request rate limiting.
//!
//! - [`RateLimitGuard`] limits per client IP. A route opts in by taking the
//!   guard, which spends a token from the bucket named by its type parameter
//!   ([`Auth`] or [`Api`]) and fails with 429 when that bucket is empty. Routes
//!   without the guard (health probes, the web UI's reads) spend nothing, so
//!   browsing the dashboard cannot starve a scanner behind the same NAT.
//! - [`ApiUserLimiter`] limits per account on the authenticated API.
//! - [`TestFireLimiter`] limits webhook test-fires per account.

use std::{marker::PhantomData, net::IpAddr, num::NonZeroU32};

use governor::{Quota, RateLimiter, clock::DefaultClock, state::keyed::DefaultKeyedStateStore};
use rocket::{Request, http::Status, outcome::Outcome};

type KeyedLimiter = RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>;

const TEST_FIRE_PER_MINUTE: NonZeroU32 = NonZeroU32::new(10).unwrap();

/// Per-IP token buckets, held as managed state and spent by [`RateLimitGuard`].
pub struct IpRateLimits {
    /// Auth endpoints (login, OIDC callback).
    auth: KeyedLimiter,
    /// Other limited API endpoints (reading ingest).
    api: KeyedLimiter,
}

impl IpRateLimits {
    pub const AUTH_PER_MINUTE: NonZeroU32 = NonZeroU32::new(20).unwrap();
    pub const API_PER_MINUTE: NonZeroU32 = NonZeroU32::new(300).unwrap();

    pub fn new(auth_per_minute: NonZeroU32, api_per_minute: NonZeroU32) -> Self {
        Self {
            auth: RateLimiter::keyed(Quota::per_minute(auth_per_minute)),
            api: RateLimiter::keyed(Quota::per_minute(api_per_minute)),
        }
    }
}

impl Default for IpRateLimits {
    fn default() -> Self {
        Self::new(Self::AUTH_PER_MINUTE, Self::API_PER_MINUTE)
    }
}

/// Selects which [`IpRateLimits`] bucket a [`RateLimitGuard`] spends from.
pub trait Bucket: Send + Sync + 'static {
    fn limiter(limits: &IpRateLimits) -> &KeyedLimiter;
}

/// The auth bucket: 20 requests/min per IP by default.
pub struct Auth;
/// The API bucket: 300 requests/min per IP by default.
pub struct Api;

impl Bucket for Auth {
    fn limiter(limits: &IpRateLimits) -> &KeyedLimiter {
        &limits.auth
    }
}

impl Bucket for Api {
    fn limiter(limits: &IpRateLimits) -> &KeyedLimiter {
        &limits.api
    }
}

fn client_ip(req: &Request<'_>) -> IpAddr {
    req.client_ip().unwrap_or(IpAddr::from([127, 0, 0, 1]))
}

/// Request guard that spends one token from bucket `B` for the client IP and
/// fails with 429 when none is left. Declare it before other guards so refused
/// requests are rejected before any authentication work. If no
/// [`IpRateLimits`] is managed (some tests), requests are allowed.
pub struct RateLimitGuard<B: Bucket>(PhantomData<B>);

#[rocket::async_trait]
impl<'r, B: Bucket> rocket::request::FromRequest<'r> for RateLimitGuard<B> {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let allowed = req
            .rocket()
            .state::<IpRateLimits>()
            .is_none_or(|limits| B::limiter(limits).check_key(&client_ip(req)).is_ok());
        if allowed {
            Outcome::Success(RateLimitGuard(PhantomData))
        } else {
            crate::metrics::rate_limited(crate::metrics::Limiter::Ip);
            Outcome::Error((Status::TooManyRequests, ()))
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
        let allowed = req
            .rocket()
            .state::<ApiUserLimiter>()
            .is_none_or(|l| l.0.check_key(&user).is_ok());
        if !allowed {
            crate::metrics::rate_limited(crate::metrics::Limiter::User);
        }
        allowed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::{get, local::asynchronous::Client, routes};
    use std::net::IpAddr;

    #[get("/auth")]
    fn auth_route(_rl: RateLimitGuard<Auth>) {}

    #[get("/api")]
    fn api_route(_rl: RateLimitGuard<Api>) {}

    #[get("/open")]
    fn open_route() {}

    async fn limited_client(auth: u32, api: u32) -> Client {
        let rocket = rocket::build()
            .manage(IpRateLimits::new(
                NonZeroU32::new(auth).unwrap(),
                NonZeroU32::new(api).unwrap(),
            ))
            .mount("/", routes![auth_route, api_route, open_route]);
        Client::tracked(rocket).await.unwrap()
    }

    async fn status_from(c: &Client, path: &str, ip: &str) -> Status {
        c.get(path)
            .remote(format!("{ip}:4000").parse().unwrap())
            .dispatch()
            .await
            .status()
    }

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

    #[rocket::async_test]
    async fn unguarded_routes_spend_no_tokens() {
        let c = limited_client(2, 2).await;
        for _ in 0..20 {
            assert_eq!(status_from(&c, "/open", "10.0.0.1").await, Status::Ok);
        }
        assert_eq!(status_from(&c, "/api", "10.0.0.1").await, Status::Ok);
        assert_eq!(status_from(&c, "/api", "10.0.0.1").await, Status::Ok);
        assert_eq!(
            status_from(&c, "/api", "10.0.0.1").await,
            Status::TooManyRequests,
            "the guarded route still enforces its own quota"
        );
    }

    #[rocket::async_test]
    async fn guarded_routes_refuse_after_quota_per_ip() {
        let c = limited_client(3, 3).await;
        for _ in 0..3 {
            assert_eq!(status_from(&c, "/auth", "10.0.0.2").await, Status::Ok);
        }
        assert_eq!(
            status_from(&c, "/auth", "10.0.0.2").await,
            Status::TooManyRequests
        );
        assert_eq!(
            status_from(&c, "/auth", "10.0.0.3").await,
            Status::Ok,
            "another IP has its own bucket"
        );
    }

    #[rocket::async_test]
    async fn auth_and_api_buckets_are_independent() {
        let c = limited_client(1, 1).await;
        assert_eq!(status_from(&c, "/auth", "10.0.0.4").await, Status::Ok);
        assert_eq!(
            status_from(&c, "/auth", "10.0.0.4").await,
            Status::TooManyRequests
        );
        assert_eq!(
            status_from(&c, "/api", "10.0.0.4").await,
            Status::Ok,
            "an empty auth bucket does not block the API bucket"
        );
        assert_eq!(
            status_from(&c, "/api", "10.0.0.4").await,
            Status::TooManyRequests
        );
    }

    #[rocket::async_test]
    async fn guard_allows_when_no_limits_are_managed() {
        let rocket = rocket::build().mount("/", routes![api_route]);
        let c = Client::tracked(rocket).await.unwrap();
        for _ in 0..5 {
            assert_eq!(status_from(&c, "/api", "10.0.0.5").await, Status::Ok);
        }
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
