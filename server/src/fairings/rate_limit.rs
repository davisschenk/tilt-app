//! Request rate limiting.
//!
//! - [`RateLimitGuard`] limits per client IP. A route opts in by taking the
//!   guard, which spends a token from the bucket named by its type parameter
//!   ([`Auth`] or [`Api`]) and fails with 429 when that bucket is empty. Routes
//!   without the guard (health probes, the web UI's reads) spend nothing, so
//!   browsing the dashboard cannot starve a scanner behind the same NAT.
//! - [`ApiUserLimiter`] limits per account on the authenticated API.
//! - [`TestFireLimiter`] limits webhook test-fires per account.
//! - [`LimiterHousekeeping`] periodically drops idle keys so the limiters do
//!   not keep an entry for every address or account ever seen.

use std::{
    hash::Hash, marker::PhantomData, net::IpAddr, num::NonZeroU32, sync::Arc, time::Duration,
};

use governor::{Quota, RateLimiter, clock::DefaultClock, state::keyed::DefaultKeyedStateStore};
use rocket::{
    Ignite, Orbit, Request, Rocket,
    fairing::{Fairing, Info, Kind},
    http::Status,
    outcome::Outcome,
};
use sea_orm::DatabaseConnection;

use crate::metrics::{self, Limiter};

type KeyedLimiter<K> = RateLimiter<K, DefaultKeyedStateStore<K>, DefaultClock>;

fn per_minute<K: Hash + Eq + Clone>(n: NonZeroU32) -> Arc<KeyedLimiter<K>> {
    Arc::new(RateLimiter::keyed(Quota::per_minute(n)))
}

/// Spend a token for `key`; on refusal, count it against `which`.
fn check<K: Hash + Eq + Clone>(limiter: &KeyedLimiter<K>, key: &K, which: Limiter) -> bool {
    let allowed = limiter.check_key(key).is_ok();
    if !allowed {
        metrics::rate_limited(which);
    }
    allowed
}

/// Forget keys whose buckets have refilled; returns how many were dropped.
fn prune<K: Hash + Eq + Clone>(limiter: &KeyedLimiter<K>) -> u64 {
    let before = limiter.len();
    limiter.retain_recent();
    limiter.shrink_to_fit();
    before.saturating_sub(limiter.len()) as u64
}

const AUTH_PER_MINUTE: NonZeroU32 = NonZeroU32::new(20).unwrap();
const API_PER_MINUTE: NonZeroU32 = NonZeroU32::new(300).unwrap();
const TEST_FIRE_PER_MINUTE: NonZeroU32 = NonZeroU32::new(10).unwrap();
const PRUNE_EVERY: Duration = Duration::from_secs(5 * 60);

/// Per-IP token buckets, held as managed state and spent by [`RateLimitGuard`].
#[derive(Clone)]
pub struct IpRateLimits {
    /// Auth endpoints (login, OIDC callback).
    auth: Arc<KeyedLimiter<IpAddr>>,
    /// Other limited API endpoints (reading ingest).
    api: Arc<KeyedLimiter<IpAddr>>,
}

impl IpRateLimits {
    pub fn new(auth_per_minute: NonZeroU32, api_per_minute: NonZeroU32) -> Self {
        Self {
            auth: per_minute(auth_per_minute),
            api: per_minute(api_per_minute),
        }
    }

    fn prune(&self) -> u64 {
        prune(&self.auth) + prune(&self.api)
    }
}

impl Default for IpRateLimits {
    fn default() -> Self {
        Self::new(AUTH_PER_MINUTE, API_PER_MINUTE)
    }
}

/// Selects which [`IpRateLimits`] bucket a [`RateLimitGuard`] spends from.
pub trait Bucket: Send + Sync + 'static {
    fn limiter(limits: &IpRateLimits) -> &KeyedLimiter<IpAddr>;
}

/// The auth bucket: 20 requests/min per IP by default.
pub struct Auth;
/// The API bucket: 300 requests/min per IP by default.
pub struct Api;

impl Bucket for Auth {
    fn limiter(limits: &IpRateLimits) -> &KeyedLimiter<IpAddr> {
        &limits.auth
    }
}

impl Bucket for Api {
    fn limiter(limits: &IpRateLimits) -> &KeyedLimiter<IpAddr> {
        &limits.api
    }
}

fn client_ip(req: &Request<'_>) -> IpAddr {
    req.client_ip().unwrap_or(IpAddr::from([127, 0, 0, 1]))
}

/// Request guard that spends one token from bucket `B` for the client IP and
/// fails with 429 when none is left. Declare it before other guards so refused
/// requests are rejected before any authentication work. Rocket refuses to
/// launch if a route takes it but [`IpRateLimits`] is not managed.
pub struct RateLimitGuard<B: Bucket>(PhantomData<B>);

#[rocket::async_trait]
impl<'r, B: Bucket> rocket::request::FromRequest<'r> for RateLimitGuard<B> {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let limits = req
            .rocket()
            .state::<IpRateLimits>()
            .expect("IpRateLimits is managed (checked by the Sentinel at launch)");
        if check(B::limiter(limits), &client_ip(req), Limiter::Ip) {
            Outcome::Success(RateLimitGuard(PhantomData))
        } else {
            Outcome::Error((Status::TooManyRequests, ()))
        }
    }
}

impl<B: Bucket> rocket::Sentinel for RateLimitGuard<B> {
    fn abort(rocket: &Rocket<Ignite>) -> bool {
        let missing = rocket.state::<IpRateLimits>().is_none();
        if missing {
            tracing::error!("a route takes RateLimitGuard but IpRateLimits is not managed");
        }
        missing
    }
}

/// Per-user limiter for endpoints that make the server send a request on the
/// user's behalf (webhook test-fire), so one account cannot use them as a cannon.
#[derive(Clone)]
pub struct TestFireLimiter(Arc<KeyedLimiter<uuid::Uuid>>);

impl TestFireLimiter {
    pub fn new() -> Self {
        Self(per_minute(TEST_FIRE_PER_MINUTE))
    }

    /// True if `user` may fire another test right now.
    pub fn allow(&self, user: uuid::Uuid) -> bool {
        self.0.check_key(&user).is_ok()
    }
}

/// Per-user request limiter for the authenticated API. The IP limiter above only
/// guards a few routes and keys on a shared address; this one follows the
/// account, whichever network or API key the request comes from.
#[derive(Clone)]
pub struct ApiUserLimiter(Arc<KeyedLimiter<uuid::Uuid>>);

impl ApiUserLimiter {
    /// Requests per minute each account may make. The web UI polls a handful of
    /// endpoints every 30s, so the default leaves a wide margin.
    pub const DEFAULT_PER_MINUTE: NonZeroU32 = NonZeroU32::new(600).unwrap();

    pub fn new(per_minute_limit: NonZeroU32) -> Self {
        Self(per_minute(per_minute_limit))
    }

    /// True if `user` may make another request right now. If no limiter is
    /// installed (some tests), requests are allowed.
    pub fn allow(req: &Request<'_>, user: uuid::Uuid) -> bool {
        req.rocket()
            .state::<ApiUserLimiter>()
            .is_none_or(|l| check(&l.0, &user, Limiter::User))
    }
}

/// Every few minutes, drops limiter entries for keys that have gone idle long
/// enough for their bucket to refill, so memory tracks active clients rather
/// than every client ever seen.
pub struct LimiterHousekeeping;

#[rocket::async_trait]
impl Fairing for LimiterHousekeeping {
    fn info(&self) -> Info {
        Info {
            name: "Rate Limiter Housekeeping",
            kind: Kind::Liftoff,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        let db = rocket
            .state::<DatabaseConnection>()
            .expect("DatabaseConnection not managed")
            .clone();
        let ip = rocket.state::<IpRateLimits>().cloned();
        let user = rocket.state::<ApiUserLimiter>().cloned();
        let fire = rocket.state::<TestFireLimiter>().cloned();
        super::periodic::spawn_every(db, "rate limiter entries", PRUNE_EVERY, move |_| {
            let n = ip.as_ref().map_or(0, IpRateLimits::prune)
                + user.as_ref().map_or(0, |l| prune(&l.0))
                + fire.as_ref().map_or(0, |l| prune(&l.0));
            async move { Ok(n) }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::{get, local::asynchronous::Client, routes};

    #[get("/auth")]
    fn auth_route(_rl: RateLimitGuard<Auth>) {}

    #[get("/api")]
    fn api_route(_rl: RateLimitGuard<Api>) {}

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
    async fn guarded_routes_refuse_after_quota_per_ip_and_bucket() {
        let two = NonZeroU32::new(2).unwrap();
        let rocket = rocket::build()
            .manage(IpRateLimits::new(two, two))
            .mount("/", routes![auth_route, api_route]);
        let c = Client::tracked(rocket).await.unwrap();
        for _ in 0..2 {
            assert_eq!(status_from(&c, "/auth", "10.0.0.1").await, Status::Ok);
        }
        assert_eq!(
            status_from(&c, "/auth", "10.0.0.1").await,
            Status::TooManyRequests
        );
        assert_eq!(
            status_from(&c, "/auth", "10.0.0.2").await,
            Status::Ok,
            "another IP has its own bucket"
        );
        for _ in 0..2 {
            assert_eq!(
                status_from(&c, "/api", "10.0.0.1").await,
                Status::Ok,
                "an empty auth bucket does not block the API bucket"
            );
        }
        assert_eq!(
            status_from(&c, "/api", "10.0.0.1").await,
            Status::TooManyRequests
        );
    }

    #[rocket::async_test]
    async fn launch_fails_if_guarded_route_has_no_limits() {
        let rocket = rocket::build().mount("/", routes![api_route]);
        let err = Client::tracked(rocket).await.err().expect("launch aborts");
        assert!(
            matches!(err.kind(), rocket::error::ErrorKind::SentinelAborts(_)),
            "{err:?}"
        );
    }

    #[test]
    fn prune_drops_refilled_keys() {
        // 60,000/min refills a spent token within a millisecond.
        let l = per_minute::<IpAddr>(NonZeroU32::new(60_000).unwrap());
        assert!(l.check_key(&IpAddr::from([10, 0, 0, 1])).is_ok());
        assert!(l.check_key(&IpAddr::from([10, 0, 0, 2])).is_ok());
        assert_eq!(l.len(), 2);
        std::thread::sleep(Duration::from_millis(20));
        assert_eq!(prune(&l), 2);
        assert!(l.is_empty());
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
