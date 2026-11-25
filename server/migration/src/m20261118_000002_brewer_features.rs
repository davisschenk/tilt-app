use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(r#"
 CREATE TABLE brewer_preferences (owner_id uuid PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE, payload jsonb NOT NULL);
 CREATE TABLE scanners (id uuid PRIMARY KEY, owner_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE, device_id varchar(64) NOT NULL, payload jsonb NOT NULL, last_seen_at timestamptz NOT NULL, UNIQUE(owner_id, device_id));
 CREATE TABLE brew_reports (id uuid PRIMARY KEY, owner_id uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE, brew_id uuid NOT NULL REFERENCES brews(id) ON DELETE CASCADE, token varchar(64) NOT NULL UNIQUE, payload jsonb NOT NULL, created_at timestamptz NOT NULL);
 ALTER TABLE readings ADD COLUMN scanner_id uuid REFERENCES scanners(id) ON DELETE SET NULL;
 CREATE INDEX readings_scanner ON readings(scanner_id) WHERE scanner_id IS NOT NULL;
 CREATE INDEX brew_reports_brew ON brew_reports(brew_id);
 "#).await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("DROP INDEX IF EXISTS readings_scanner; ALTER TABLE readings DROP COLUMN scanner_id; DROP TABLE brew_reports; DROP TABLE scanners; DROP TABLE brewer_preferences;").await?;
        Ok(())
    }
}
