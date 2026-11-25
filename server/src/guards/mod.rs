pub mod api_key;
pub mod auth_or_api_key;
pub mod current_user;

use rocket::Request;

/// The token from an `Authorization: Bearer <token>` header, if present.
pub fn bearer_token<'r>(req: &'r Request<'_>) -> Option<&'r str> {
    req.headers()
        .get_one("Authorization")
        .and_then(|v| v.strip_prefix("Bearer "))
}
