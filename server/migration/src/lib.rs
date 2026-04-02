pub use sea_orm_migration::prelude::*;

mod m20260215_000001_create_hydrometers;
mod m20260215_000002_create_brews;
mod m20260215_000003_create_readings;
mod m20260219_012142_create_user_sessions;
mod m20260219_012410_create_api_keys;
mod m20260301_000001_create_alert_targets;
mod m20260301_000002_create_alert_rules;
mod m20260316_000001_create_brew_events;
mod m20260316_000002_add_window_hours_to_alert_rules;
mod m20260317_071445_drop_abv_from_brews;
mod m20260402_005912_add_nutrient_schedule_fields_to_brews;

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
            Box::new(m20260301_000001_create_alert_targets::Migration),
            Box::new(m20260301_000002_create_alert_rules::Migration),
            Box::new(m20260316_000001_create_brew_events::Migration),
            Box::new(m20260316_000002_add_window_hours_to_alert_rules::Migration),
            Box::new(m20260317_071445_drop_abv_from_brews::Migration),
            Box::new(m20260402_005912_add_nutrient_schedule_fields_to_brews::Migration),
        ]
    }
}
