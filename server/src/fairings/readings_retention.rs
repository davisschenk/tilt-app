use rocket::{
    Rocket,
    fairing::{Fairing, Info, Kind},
};
use sea_orm::DatabaseConnection;
use std::time::Duration;

/// Hourly purge of readings not linked to any brew, older than `days`.
/// Only attached when `READINGS_RETENTION_DAYS` is set to a positive number;
/// see `services::retention_service` for exactly what is deleted.
pub struct ReadingsRetention {
    pub days: u32,
}

#[rocket::async_trait]
impl Fairing for ReadingsRetention {
    fn info(&self) -> Info {
        Info {
            name: "Readings Retention",
            kind: Kind::Liftoff,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<rocket::Orbit>) {
        let db = rocket
            .state::<DatabaseConnection>()
            .expect("DatabaseConnection not managed")
            .clone();
        let days = self.days;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(3600));
            loop {
                interval.tick().await;
                match crate::services::retention_service::purge_unlinked_readings(&db, days).await {
                    Ok(n) => {
                        if n > 0 {
                            tracing::info!(
                                deleted = n,
                                retention_days = days,
                                "Deleted readings not linked to any brew"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Readings retention cleanup failed");
                    }
                }
            }
        });
    }
}
