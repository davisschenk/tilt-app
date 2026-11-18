use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// The readings table is the one that grows without bound. Every real query
/// filters by a device or a brew *and* a time range, and the ingest cap counts
/// recent arrivals per device, so those get composite indexes. The old
/// single-column hydrometer/brew indexes are prefixes of the new ones, so they
/// are dropped to keep inserts cheap.
///
/// Note: plain `CREATE INDEX` blocks writes to `readings` while it builds. That
/// is instant at current sizes; on a very large table, create these by hand with
/// `CREATE INDEX CONCURRENTLY` before running the migration (the `IF NOT EXISTS`
/// makes it a no-op afterwards).
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for sql in [
            "CREATE INDEX IF NOT EXISTS idx_readings_hydrometer_recorded ON readings (hydrometer_id, recorded_at)",
            "CREATE INDEX IF NOT EXISTS idx_readings_brew_recorded ON readings (brew_id, recorded_at)",
            "CREATE INDEX IF NOT EXISTS idx_readings_hydrometer_created ON readings (hydrometer_id, created_at)",
            "DROP INDEX IF EXISTS idx_readings_hydrometer_id",
            "DROP INDEX IF EXISTS idx_readings_brew_id",
        ] {
            db.execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        for sql in [
            "CREATE INDEX IF NOT EXISTS idx_readings_hydrometer_id ON readings (hydrometer_id)",
            "CREATE INDEX IF NOT EXISTS idx_readings_brew_id ON readings (brew_id)",
            "DROP INDEX IF EXISTS idx_readings_hydrometer_recorded",
            "DROP INDEX IF EXISTS idx_readings_brew_recorded",
            "DROP INDEX IF EXISTS idx_readings_hydrometer_created",
        ] {
            db.execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
