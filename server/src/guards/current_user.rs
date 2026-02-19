use rocket::{
    Request,
    http::Status,
    request::{FromRequest, Outcome},
};
use sea_orm::DatabaseConnection;
use uuid::Uuid;

use crate::services::sessions;

#[derive(Debug, Clone)]
pub struct CurrentUser {
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
                Outcome::Success(CurrentUser {
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
