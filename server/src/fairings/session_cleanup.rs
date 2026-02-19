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

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600));
            loop {
                interval.tick().await;
                match crate::services::sessions::delete_expired_sessions(&db).await {
                    Ok(n) => {
                        if n > 0 {
                            tracing::info!(deleted = n, "Cleaned up expired sessions");
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Session cleanup failed");
                    }
                }
            }
        });
    }
}
