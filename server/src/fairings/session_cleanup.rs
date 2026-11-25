use rocket::{
    Rocket,
    fairing::{Fairing, Info, Kind},
};
use sea_orm::DatabaseConnection;
use std::time::Duration;

pub struct SessionCleanup;

#[rocket::async_trait]
impl Fairing for SessionCleanup {
    fn info(&self) -> Info {
        Info {
            name: "Session Cleanup",
            kind: Kind::Liftoff,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<rocket::Orbit>) {
        let db = rocket
            .state::<DatabaseConnection>()
            .expect("DatabaseConnection not managed")
            .clone();
        super::periodic::spawn_every(
            db,
            "expired sessions",
            Duration::from_secs(3600),
            |db| async move { crate::services::sessions::delete_expired_sessions(&db).await },
        );
    }
}
