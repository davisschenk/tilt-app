use rocket::{
    Request,
    http::Status,
    request::{FromRequest, Outcome},
};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::auth_mode::{AuthMode, dev_user};
use crate::services::{sessions, user_service};

#[derive(Debug, Clone)]
pub struct CurrentUser {
    /// Primary key of the `users` row; scope every query by this.
    pub user_id: Uuid,
    pub session_id: Uuid,
    pub user_sub: String,
    pub email: String,
    pub name: String,
}

pub const SESSION_COOKIE: &str = "session_id";

#[rocket::async_trait]
impl<'r> FromRequest<'r> for CurrentUser {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Auth disabled: inject a fixed dev user. All requests succeed.
        if AuthMode::of(req).is_disabled() {
            let db = match req.guard::<&rocket::State<DatabaseConnection>>().await {
                Outcome::Success(db) => db,
                _ => return Outcome::Error((Status::InternalServerError, ())),
            };
            return match user_service::resolve(db, dev_user::SUB, dev_user::EMAIL, dev_user::NAME)
                .await
            {
                Ok(u) => Outcome::Success(CurrentUser {
                    user_id: u.id,
                    session_id: dev_user::SESSION_ID,
                    user_sub: dev_user::SUB.to_string(),
                    email: dev_user::EMAIL.to_string(),
                    name: dev_user::NAME.to_string(),
                }),
                Err(_) => Outcome::Error((Status::InternalServerError, ())),
            };
        }

        let cookie = req.cookies().get_private(SESSION_COOKIE);
        let session_id_str = match cookie {
            Some(c) => c.value().to_string(),
            None => return Outcome::Error((Status::Unauthorized, ())),
        };

        let session_id = match Uuid::parse_str(&session_id_str) {
            Ok(id) => id,
            Err(_) => return Outcome::Error((Status::Unauthorized, ())),
        };

        let db = match req.guard::<&rocket::State<DatabaseConnection>>().await {
            Outcome::Success(db) => db,
            _ => return Outcome::Error((Status::InternalServerError, ())),
        };

        match sessions::get_session_by_id(db, session_id).await {
            Ok(Some(session)) => {
                let now = chrono::Utc::now().fixed_offset();
                if session.expires_at < now {
                    return Outcome::Error((Status::Unauthorized, ()));
                }
                let _ = sessions::touch_session(db, session_id).await;
                let user = match user_service::resolve(
                    db,
                    &session.user_sub,
                    &session.email,
                    &session.name,
                )
                .await
                {
                    Ok(u) => u,
                    Err(_) => return Outcome::Error((Status::InternalServerError, ())),
                };
                Outcome::Success(CurrentUser {
                    user_id: user.id,
                    session_id,
                    user_sub: session.user_sub,
                    email: session.email,
                    name: session.name,
                })
            }
            Ok(None) => Outcome::Error((Status::Unauthorized, ())),
            Err(_) => Outcome::Error((Status::InternalServerError, ())),
        }
    }
}
