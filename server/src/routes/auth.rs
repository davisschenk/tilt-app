use openidconnect::{AuthorizationCode, Nonce, PkceCodeVerifier, TokenResponse, core::CoreTokenResponse};
use rocket::{
    State, get, post,
    http::{Cookie, CookieJar, SameSite, Status},
    response::Redirect,
    serde::json::Json,
    time::Duration,
};
use sea_orm::DatabaseConnection;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    guards::current_user::{CurrentUser, SESSION_COOKIE},
    oidc::OidcState,
    services::sessions::{self, CreateSessionParams},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthMeResponse {
    pub user_sub: String,
    pub email: String,
    pub name: String,
}

#[get("/auth/login")]
pub async fn login(
    oidc: Option<&State<OidcState>>,
    cookies: &CookieJar<'_>,
) -> Result<Redirect, (Status, Json<serde_json::Value>)> {
    let oidc = oidc.ok_or_else(|| {
        (
            Status::ServiceUnavailable,
            Json(serde_json::json!({ "error": "authentication not configured" })),
        )
    })?;
    let (auth_url, csrf_token, nonce, pkce_verifier) = oidc.authorization_url();

    let nonce_val = nonce.secret().clone();
    let csrf_val = csrf_token.secret().clone();
    let verifier_val = pkce_verifier.secret().clone();

    cookies.add_private(
        Cookie::build(("oidc_nonce", nonce_val))
            .same_site(SameSite::Lax)
            .max_age(Duration::minutes(10))
            .build(),
    );
    cookies.add_private(
        Cookie::build(("oidc_csrf", csrf_val))
            .same_site(SameSite::Lax)
            .max_age(Duration::minutes(10))
            .build(),
    );
    cookies.add_private(
        Cookie::build(("oidc_pkce", verifier_val))
            .same_site(SameSite::Lax)
            .max_age(Duration::minutes(10))
            .build(),
    );

    Ok(Redirect::to(auth_url.to_string()))
}

#[get("/auth/callback?<code>&<state>")]
pub async fn callback(
    code: String,
    state: String,
    oidc: Option<&State<OidcState>>,
    db: &State<DatabaseConnection>,
    cookies: &CookieJar<'_>,
) -> Result<Redirect, (Status, Json<serde_json::Value>)> {
    let oidc = oidc.ok_or_else(|| {
        (
            Status::ServiceUnavailable,
            Json(serde_json::json!({ "error": "authentication not configured" })),
        )
    })?;
    let err = |msg: &str| {
        (
            Status::BadRequest,
            Json(serde_json::json!({ "error": msg })),
        )
    };

    let stored_csrf = cookies
        .get_private("oidc_csrf")
        .map(|c| c.value().to_string())
        .ok_or_else(|| err("missing CSRF state"))?;

    if stored_csrf != state {
        return Err(err("CSRF state mismatch"));
    }

    let stored_nonce = cookies
        .get_private("oidc_nonce")
        .map(|c| c.value().to_string())
        .ok_or_else(|| err("missing nonce"))?;

    let stored_pkce = cookies
        .get_private("oidc_pkce")
        .map(|c| c.value().to_string())
        .ok_or_else(|| err("missing PKCE verifier"))?;

    cookies.remove_private("oidc_csrf");
    cookies.remove_private("oidc_nonce");
    cookies.remove_private("oidc_pkce");

    let oidc = oidc.inner();

    let token_response: CoreTokenResponse = oidc
        .client
        .exchange_code(AuthorizationCode::new(code))
        .map_err(|e| err(&format!("exchange_code error: {e}")))?
        .set_pkce_verifier(PkceCodeVerifier::new(stored_pkce))
        .request_async(&oidc.http_client)
        .await
        .map_err(|e| err(&format!("token exchange failed: {e}")))?;

    let id_token = token_response
        .id_token()
        .ok_or_else(|| err("no ID token in response"))?;

    let verifier = oidc.client.id_token_verifier();
    let nonce = Nonce::new(stored_nonce);
    let claims = id_token
        .claims(&verifier, &nonce)
        .map_err(|e| err(&format!("ID token verification failed: {e}")))?;

    let user_sub = claims.subject().to_string();
    let email = claims
        .email()
        .map(|e| e.as_str().to_string())
        .unwrap_or_else(|| user_sub.clone());
    let name = claims
        .name()
        .and_then(|n| n.get(None))
        .map(|n| n.as_str().to_string())
        .unwrap_or_else(|| email.clone());

    let id_token_str = id_token.to_string();
    let id_token_hash = format!("{:x}", Sha256::digest(id_token_str.as_bytes()));

    let session = sessions::create_session(
        db,
        CreateSessionParams {
            user_sub,
            email,
            name,
            id_token_hash,
            expires_in_secs: 86400,
        },
    )
    .await
    .map_err(|e| {
        (
            Status::InternalServerError,
            Json(serde_json::json!({ "error": format!("session creation failed: {e}") })),
        )
    })?;

    cookies.add_private(
        Cookie::build((SESSION_COOKIE, session.id.to_string()))
            .same_site(SameSite::Lax)
            .max_age(Duration::days(1))
            .http_only(true)
            .build(),
    );

    let frontend_url = std::env::var("FRONTEND_URL").unwrap_or_else(|_| "/".to_string());
    Ok(Redirect::to(frontend_url))
}

#[post("/auth/logout")]
pub async fn logout(
    user: CurrentUser,
    db: &State<DatabaseConnection>,
    cookies: &CookieJar<'_>,
) -> Json<serde_json::Value> {
    let _ = sessions::delete_session(db, user.session_id).await;
    cookies.remove_private(SESSION_COOKIE);
    Json(serde_json::json!({ "ok": true }))
}

#[get("/auth/me")]
pub async fn me(user: CurrentUser) -> Json<AuthMeResponse> {
    Json(AuthMeResponse {
        user_sub: user.user_sub,
        email: user.email,
        name: user.name,
    })
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![login, callback, logout, me]
}
