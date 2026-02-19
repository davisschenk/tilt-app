pub use sea_orm_migration::prelude::*;

mod m20260215_000001_create_hydrometers;
mod m20260215_000002_create_brews;
mod m20260215_000003_create_readings;
mod m20260219_012142_create_user_sessions;
mod m20260219_012410_create_api_keys;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260215_000001_create_hydrometers::Migration),
            Box::new(m20260215_000002_create_brews::Migration),
            Box::new(m20260215_000003_create_readings::Migration),
            Box::new(m20260219_012142_create_user_sessions::Migration),
            Box::new(m20260219_012410_create_api_keys::Migration),
        ]
    }
}
