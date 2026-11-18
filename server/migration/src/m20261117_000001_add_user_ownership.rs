use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// Tables that gain an `owner_id` column. Child tables (readings, brew_events,
/// event_attachments, alert_rules) inherit ownership through their parent.
const OWNED_TABLES: [&str; 4] = ["hydrometers", "brews", "alert_targets", "api_keys"];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            "CREATE TABLE users (
                id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
                oidc_sub varchar NOT NULL UNIQUE,
                email varchar NOT NULL,
                name varchar NOT NULL,
                created_at timestamptz NOT NULL DEFAULT now()
            )",
        )
        .await?;

        // Backfill users from existing sessions. Profile fields come from the latest
        // session; `created_at` is the user's first session, so "earliest user"
        // below means the person who has been here longest, not a random tiebreak.
        db.execute_unprepared(
            "INSERT INTO users (oidc_sub, email, name, created_at)
             SELECT DISTINCT ON (user_sub) user_sub, email, name,
                    MIN(created_at) OVER (PARTITION BY user_sub)
             FROM user_sessions
             ORDER BY user_sub, created_at DESC",
        )
        .await?;

        // ...and from API key creators that never had a surviving session.
        db.execute_unprepared(
            "INSERT INTO users (oidc_sub, email, name, created_at)
             SELECT created_by, created_by, created_by, MIN(created_at)
             FROM api_keys
             WHERE created_by NOT IN (SELECT oidc_sub FROM users)
             GROUP BY created_by",
        )
        .await?;

        // Pre-existing data with nobody to own it (AUTH_MODE=disabled installs)
        // goes to the dev user so the NOT NULL below cannot fail.
        db.execute_unprepared(
            "INSERT INTO users (oidc_sub, email, name)
             SELECT 'dev-local', 'dev@local', 'Dev User'
             WHERE NOT EXISTS (SELECT 1 FROM users)
               AND (EXISTS (SELECT 1 FROM hydrometers)
                 OR EXISTS (SELECT 1 FROM brews)
                 OR EXISTS (SELECT 1 FROM alert_targets))",
        )
        .await?;

        for table in OWNED_TABLES {
            db.execute_unprepared(&format!("ALTER TABLE {table} ADD COLUMN owner_id uuid"))
                .await?;
        }

        // API keys belong to whoever created them.
        db.execute_unprepared(
            "UPDATE api_keys SET owner_id = u.id FROM users u WHERE u.oidc_sub = api_keys.created_by",
        )
        .await?;

        // Everything else goes to the earliest user (the original admin).
        for table in ["hydrometers", "brews", "alert_targets"] {
            db.execute_unprepared(&format!(
                "UPDATE {table} SET owner_id = (SELECT id FROM users ORDER BY created_at, id LIMIT 1)"
            ))
            .await?;
        }

        for table in OWNED_TABLES {
            db.execute_unprepared(&format!(
                "ALTER TABLE {table}
                   ALTER COLUMN owner_id SET NOT NULL,
                   ADD CONSTRAINT fk_{table}_owner_id FOREIGN KEY (owner_id)
                     REFERENCES users(id) ON DELETE CASCADE"
            ))
            .await?;
            db.execute_unprepared(&format!(
                "CREATE INDEX idx_{table}_owner_id ON {table} (owner_id)"
            ))
            .await?;
        }

        // A Tilt color is unique per owner, not globally.
        db.execute_unprepared(
            "ALTER TABLE hydrometers DROP CONSTRAINT IF EXISTS hydrometers_color_key",
        )
        .await?;
        db.execute_unprepared(
            "CREATE UNIQUE INDEX idx_hydrometers_owner_color ON hydrometers (owner_id, color)",
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared("DROP INDEX IF EXISTS idx_hydrometers_owner_color")
            .await?;
        // Restoring global uniqueness fails if two owners share a color; that is
        // the correct outcome, since the old schema cannot represent that data.
        db.execute_unprepared(
            "ALTER TABLE hydrometers ADD CONSTRAINT hydrometers_color_key UNIQUE (color)",
        )
        .await?;

        for table in OWNED_TABLES {
            db.execute_unprepared(&format!("DROP INDEX IF EXISTS idx_{table}_owner_id"))
                .await?;
            db.execute_unprepared(&format!(
                "ALTER TABLE {table}
                   DROP CONSTRAINT IF EXISTS fk_{table}_owner_id,
                   DROP COLUMN owner_id"
            ))
            .await?;
        }

        db.execute_unprepared("DROP TABLE users").await?;
        Ok(())
    }
}
