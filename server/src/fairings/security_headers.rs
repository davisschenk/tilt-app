use rocket::{
    Request, Response,
    fairing::{Fairing, Info, Kind},
    http::Header,
};

pub struct SecurityHeaders;

#[rocket::async_trait]
impl Fairing for SecurityHeaders {
    fn info(&self) -> Info {
        Info {
            name: "Security Headers",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _req: &'r Request<'_>, res: &mut Response<'r>) {
        res.set_header(Header::new(
            "X-Content-Type-Options",
            "nosniff",
        ));
        res.set_header(Header::new(
            "X-Frame-Options",
            "DENY",
        ));
        res.set_header(Header::new(
            "X-XSS-Protection",
            "1; mode=block",
        ));
        res.set_header(Header::new(
            "Referrer-Policy",
            "strict-origin-when-cross-origin",
        ));
        res.set_header(Header::new(
            "Permissions-Policy",
            "geolocation=(), microphone=(), camera=()",
        ));
        res.set_header(Header::new(
            "Strict-Transport-Security",
            "max-age=31536000; includeSubDomains",
        ));
        res.set_header(Header::new(
            "Content-Security-Policy",
            "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'",
        ));
    }
}
