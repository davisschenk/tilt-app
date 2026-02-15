pub use sea_orm_migration::prelude::*;

mod m20260215_000001_create_hydrometers;
mod m20260215_000002_create_brews;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20260215_000001_create_hydrometers::Migration),
            Box::new(m20260215_000002_create_brews::Migration),
        ]
    }
}
