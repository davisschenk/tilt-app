use rocket::{
    Request, Response,
    fairing::{Fairing, Info, Kind},
    http::Header,
};

/// Default policy for the SPA and the API. Verified against the production web
/// build by the Playwright suite (web/e2e/csp.spec.ts; every E2E test fails on a
/// CSP violation), so a change here or a new dependency that needs more is caught.
///
/// - script-src: the Vite build is external module scripts only — no inline
///   scripts, no eval. Cloudflare Web Analytics, when the site is proxied by
///   Cloudflare, injects an external beacon script and posts to its endpoint.
///   Fixed rather than configurable until a second deployment needs other hosts.
/// - style-src 'unsafe-inline': required. sonner injects its stylesheet as a
///   `<style>` element at runtime, and react-remove-scroll (Radix dialogs/menus)
///   injects one whose content includes the measured scrollbar width, so no hash
///   can be stable; react-md-editor's markdown preview also emits style attributes.
/// - img-src data:: inline SVG/GIF data URIs (chart exports, markdown).
/// - form-action 'self': login is a top-level GET navigation to
///   /api/v1/auth/login, which redirects to the OIDC issuer — no form posts to the
///   IdP — and every app form is handled in JS.
pub const CONTENT_SECURITY_POLICY: &str = concat!(
    "default-src 'self'; ",
    "script-src 'self' https://static.cloudflareinsights.com; ",
    "style-src 'self' 'unsafe-inline'; ",
    "img-src 'self' data:; ",
    "connect-src 'self' https://cloudflareinsights.com; ",
    "object-src 'none'; ",
    "base-uri 'self'; ",
    "frame-ancestors 'none'; ",
    "form-action 'self'",
);

/// Web Serial stays enabled for our own origin: the device-setup page flashes and
/// configures the ESP32 over it.
pub const PERMISSIONS_POLICY: &str =
    "camera=(), microphone=(), geolocation=(), payment=(), usb=(), serial=(self)";

pub struct SecurityHeaders;

/// HSTS is only meaningful (and only honoured by browsers) over HTTPS. The server
/// itself speaks plain HTTP behind a TLS-terminating proxy (Caddy, Cloudflare), so
/// trust the proxy's `X-Forwarded-Proto`; a spoofed value over plain HTTP is
/// harmless because browsers ignore HSTS received over an insecure connection.
fn is_https(req: &Request<'_>) -> bool {
    req.rocket().config().tls_enabled()
        || req
            .headers()
            .get_one("X-Forwarded-Proto")
            .and_then(|v| v.split(',').next())
            .is_some_and(|proto| proto.trim().eq_ignore_ascii_case("https"))
}

#[rocket::async_trait]
impl Fairing for SecurityHeaders {
    fn info(&self) -> Info {
        Info {
            name: "Security Headers",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, req: &'r Request<'_>, res: &mut Response<'r>) {
        res.set_header(Header::new("X-Content-Type-Options", "nosniff"));
        res.set_header(Header::new("X-Frame-Options", "DENY"));
        res.set_header(Header::new(
            "Referrer-Policy",
            "strict-origin-when-cross-origin",
        ));
        res.set_header(Header::new("Permissions-Policy", PERMISSIONS_POLICY));
        // Login is a full-page redirect, never a popup, so nothing needs an opener.
        res.set_header(Header::new("Cross-Origin-Opener-Policy", "same-origin"));
        if is_https(req) {
            res.set_header(Header::new(
                "Strict-Transport-Security",
                "max-age=31536000; includeSubDomains",
            ));
        }
        // Routes that serve user content set their own, stricter policy; keep it.
        if !res.headers().contains("Content-Security-Policy") {
            res.set_header(Header::new(
                "Content-Security-Policy",
                CONTENT_SECURITY_POLICY,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::{get, http::Header, local::asynchronous::Client, routes};

    #[get("/")]
    fn index() -> &'static str {
        "ok"
    }

    struct WithOwnCsp;
    impl<'r> rocket::response::Responder<'r, 'static> for WithOwnCsp {
        fn respond_to(self, _: &'r Request<'_>) -> rocket::response::Result<'static> {
            Response::build()
                .raw_header("Content-Security-Policy", "default-src 'none'; sandbox")
                .ok()
        }
    }

    #[get("/sandboxed")]
    fn sandboxed() -> WithOwnCsp {
        WithOwnCsp
    }

    async fn client() -> Client {
        let rocket = rocket::build()
            .attach(SecurityHeaders)
            .mount("/", routes![index, sandboxed]);
        Client::tracked(rocket).await.unwrap()
    }

    #[rocket::async_test]
    async fn sets_policy_and_no_hsts_over_plain_http() {
        let client = client().await;
        let res = client.get("/").dispatch().await;
        let h = res.headers();
        assert_eq!(
            h.get_one("Content-Security-Policy"),
            Some(CONTENT_SECURITY_POLICY)
        );
        assert_eq!(h.get_one("X-Content-Type-Options"), Some("nosniff"));
        assert_eq!(h.get_one("X-Frame-Options"), Some("DENY"));
        assert!(
            h.get_one("Permissions-Policy")
                .unwrap()
                .contains("serial=(self)")
        );
        assert!(h.get_one("Strict-Transport-Security").is_none());
        assert!(
            !CONTENT_SECURITY_POLICY
                .split(';')
                .any(|d| d.trim().starts_with("script-src") && d.contains("unsafe"))
        );
    }

    #[rocket::async_test]
    async fn hsts_when_proxy_reports_https() {
        let client = client().await;
        for proto in ["https", "HTTPS", "https, http"] {
            let res = client
                .get("/")
                .header(Header::new("X-Forwarded-Proto", proto))
                .dispatch()
                .await;
            assert!(
                res.headers().get_one("Strict-Transport-Security").is_some(),
                "{proto}"
            );
        }
        let res = client
            .get("/")
            .header(Header::new("X-Forwarded-Proto", "http"))
            .dispatch()
            .await;
        assert!(res.headers().get_one("Strict-Transport-Security").is_none());
    }

    #[rocket::async_test]
    async fn keeps_a_route_specific_policy() {
        let client = client().await;
        let res = client.get("/sandboxed").dispatch().await;
        let csps: Vec<_> = res.headers().get("Content-Security-Policy").collect();
        assert_eq!(csps, vec!["default-src 'none'; sandbox"]);
    }
}
