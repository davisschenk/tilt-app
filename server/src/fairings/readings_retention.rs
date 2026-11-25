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
        super::periodic::spawn_every(
            db,
            "unlinked readings retention",
            Duration::from_secs(3600),
            move |db| async move {
                crate::services::retention_service::purge_unlinked_readings(&db, days).await
            },
        );
    }
}
