//! Opt-in retention for readings that are not part of any brew.
//!
//! A reading belongs to a brew through `readings.brew_id`, which ingest sets to
//! the hydrometer's active brew at the time the reading arrives. Every brew view
//! (chart, analytics, report, `GET /readings?brew_id=`) selects by that column,
//! so a reading with `brew_id IS NULL` is not shown as part of any brew. Those
//! are what a hydrometer reports while idle between batches, and they are the
//! only rows this module ever deletes.
//!
//! As a second, independent guard, a reading is also kept if its hydrometer has
//! any brew whose window `[start_date, end_date-or-now]` covers the reading's
//! `recorded_at`, whatever its `brew_id` says. That protects readings that
//! should have been linked but were not (for example an ingest whose active-brew
//! lookup failed), and errs towards keeping data. Age is measured by both the
//! device-supplied `recorded_at` and the server's `created_at`, so a device clock
//! that is wrong cannot make a fresh reading look old.

use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement};
use std::time::Duration;

/// Rows removed per statement, so one run never holds long locks on `readings`.
pub const BATCH_SIZE: u64 = 5_000;
/// Pause between batches, to let ingest and queries interleave.
const BATCH_PAUSE: Duration = Duration::from_millis(100);
/// Upper bound on the setting (100 years); anything bigger is a typo.
const MAX_DAYS: u32 = 36_500;

/// Parse `READINGS_RETENTION_DAYS`. Unset, empty or `0` disables retention
/// (`Ok(None)`); a positive whole number of days enables it; anything else is a
/// configuration error the server refuses to start with.
pub fn retention_days(get: impl Fn(&str) -> Option<String>) -> Result<Option<u32>, String> {
    let Some(raw) = get("READINGS_RETENTION_DAYS") else {
        return Ok(None);
    };
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    match raw.parse::<u32>() {
        Ok(0) => Ok(None),
        Ok(days) if days <= MAX_DAYS => Ok(Some(days)),
        _ => Err(format!(
            "READINGS_RETENTION_DAYS must be a whole number of days between 0 (disabled) and \
             {MAX_DAYS}, got {raw:?}"
        )),
    }
}

/// Delete up to `limit` unlinked readings older than `cutoff`. Returns how many
/// were deleted; fewer than `limit` means there are none left.
pub async fn delete_unlinked_batch(
    db: &impl ConnectionTrait,
    cutoff: chrono::DateTime<chrono::Utc>,
    limit: u64,
) -> Result<u64, DbErr> {
    // `brew_id IS NULL AND recorded_at < $1` is served by
    // idx_readings_brew_recorded (brew_id, recorded_at); the NOT EXISTS probe
    // by idx_brews_hydrometer_id.
    let sql = r#"
        DELETE FROM readings
        WHERE id IN (
            SELECT r.id FROM readings r
            WHERE r.brew_id IS NULL
              AND r.recorded_at < $1
              AND r.created_at < $1
              AND NOT EXISTS (
                  SELECT 1 FROM brews b
                  WHERE b.hydrometer_id = r.hydrometer_id
                    AND r.recorded_at >= COALESCE(b.start_date, b.created_at)
                    AND r.recorded_at <= COALESCE(b.end_date, now())
              )
            LIMIT $2
        )
    "#;
    let result = db
        .execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [cutoff.into(), (limit as i64).into()],
        ))
        .await?;
    Ok(result.rows_affected())
}

/// Delete every unlinked reading older than `days`, in bounded batches.
pub async fn purge_unlinked_readings(db: &DatabaseConnection, days: u32) -> Result<u64, DbErr> {
    let cutoff = chrono::Utc::now() - chrono::Duration::days(i64::from(days));
    let mut total = 0;
    loop {
        let n = delete_unlinked_batch(db, cutoff, BATCH_SIZE).await?;
        total += n;
        if n < BATCH_SIZE {
            return Ok(total);
        }
        tokio::time::sleep(BATCH_PAUSE).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::entities::{brews, hydrometers, readings, users};
    use chrono::{DateTime, Utc};
    use sea_orm::{ActiveModelTrait, ConnectOptions, Database, EntityTrait, PaginatorTrait, Set};
    use sea_orm_migration::MigratorTrait;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let m: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |k| m.get(k).cloned()
    }

    #[test]
    fn unset_empty_or_zero_disables() {
        assert_eq!(retention_days(env(&[])), Ok(None));
        assert_eq!(
            retention_days(env(&[("READINGS_RETENTION_DAYS", "")])),
            Ok(None)
        );
        assert_eq!(
            retention_days(env(&[("READINGS_RETENTION_DAYS", " ")])),
            Ok(None)
        );
        assert_eq!(
            retention_days(env(&[("READINGS_RETENTION_DAYS", "0")])),
            Ok(None)
        );
    }

    #[test]
    fn positive_days_enable() {
        assert_eq!(
            retention_days(env(&[("READINGS_RETENTION_DAYS", "90")])),
            Ok(Some(90))
        );
        assert_eq!(
            retention_days(env(&[("READINGS_RETENTION_DAYS", " 30 ")])),
            Ok(Some(30))
        );
    }

    #[test]
    fn garbage_is_rejected() {
        for bad in ["-1", "abc", "1.5", "30d", "99999999999", "36501"] {
            assert!(
                retention_days(env(&[("READINGS_RETENTION_DAYS", bad)])).is_err(),
                "{bad} should be rejected"
            );
        }
    }

    struct Db {
        db: DatabaseConnection,
        admin: DatabaseConnection,
        schema: String,
    }

    async fn fresh_schema() -> Db {
        let url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must point at a Postgres the tests may create schemas in");
        let schema = format!("t_{}", Uuid::new_v4().simple());
        let admin = Database::connect(&url).await.expect("connect to Postgres");
        admin
            .execute_unprepared(&format!("CREATE SCHEMA {schema}"))
            .await
            .expect("create test schema");
        let mut opts = ConnectOptions::new(url);
        opts.set_schema_search_path(schema.clone())
            .max_connections(2);
        let db = Database::connect(opts)
            .await
            .expect("connect to test schema");
        migration::Migrator::up(&db, None)
            .await
            .expect("migrations apply to an empty schema");
        Db { db, admin, schema }
    }

    async fn user(db: &DatabaseConnection, sub: &str) -> Uuid {
        let id = Uuid::new_v4();
        users::ActiveModel {
            id: Set(id),
            oidc_sub: Set(sub.into()),
            email: Set(format!("{sub}@example.com")),
            name: Set(sub.into()),
            created_at: Set(Utc::now().into()),
        }
        .insert(db)
        .await
        .unwrap();
        id
    }

    async fn hydrometer(db: &DatabaseConnection, owner: Uuid, color: &str) -> Uuid {
        let id = Uuid::new_v4();
        hydrometers::ActiveModel {
            id: Set(id),
            owner_id: Set(owner),
            color: Set(color.into()),
            name: Set(None),
            temp_offset_f: Set(0.0),
            gravity_offset: Set(0.0),
            created_at: Set(Utc::now().into()),
            is_disabled: Set(false),
        }
        .insert(db)
        .await
        .unwrap();
        id
    }

    async fn brew(
        db: &DatabaseConnection,
        owner: Uuid,
        hydrometer: Uuid,
        status: &str,
        start: DateTime<Utc>,
        end: Option<DateTime<Utc>>,
    ) -> Uuid {
        let id = Uuid::new_v4();
        brews::ActiveModel {
            id: Set(id),
            owner_id: Set(owner),
            name: Set(format!("{status} brew")),
            style: Set(None),
            og: Set(None),
            fg: Set(None),
            target_fg: Set(None),
            status: Set(status.into()),
            start_date: Set(Some(start.into())),
            end_date: Set(end.map(Into::into)),
            notes: Set(None),
            hydrometer_id: Set(hydrometer),
            created_at: Set(start.into()),
            updated_at: Set(start.into()),
            batch_size_gallons: Set(None),
            yeast_nitrogen_requirement: Set(None),
            pitch_time: Set(None),
            nutrient_protocol: Set(None),
            yeast_strain: Set(None),
            nutrient_alert_target_id: Set(None),
        }
        .insert(db)
        .await
        .unwrap();
        id
    }

    /// Insert a reading recorded (and received) at `at`.
    async fn reading(
        db: &DatabaseConnection,
        hydrometer: Uuid,
        brew: Option<Uuid>,
        at: DateTime<Utc>,
    ) -> Uuid {
        let id = Uuid::new_v4();
        readings::ActiveModel {
            id: Set(id),
            brew_id: Set(brew),
            scanner_id: Set(None),
            hydrometer_id: Set(hydrometer),
            temperature_f: Set(68.0),
            gravity: Set(1.050),
            rssi: Set(None),
            recorded_at: Set(at.into()),
            created_at: Set(at.into()),
        }
        .insert(db)
        .await
        .unwrap();
        id
    }

    async fn exists(db: &DatabaseConnection, id: Uuid) -> bool {
        readings::Entity::find_by_id(id)
            .one(db)
            .await
            .unwrap()
            .is_some()
    }

    #[rocket::async_test]
    async fn only_old_unlinked_readings_are_deleted() {
        let Db { db, admin, schema } = fresh_schema().await;
        let now = Utc::now();
        let days = |n: i64| now - chrono::Duration::days(n);

        let a = user(&db, "user-a").await;
        let b = user(&db, "user-b").await;
        let red = hydrometer(&db, a, "Red").await;
        let green = hydrometer(&db, a, "Green").await;
        let blue = hydrometer(&db, b, "Blue").await;

        // Red: a completed brew 200..150 days ago, then idle, then an active
        // brew since 40 days ago.
        let done = brew(&db, a, red, "Completed", days(200), Some(days(150))).await;
        let active = brew(&db, a, red, "Active", days(40), None).await;
        let in_completed = reading(&db, red, Some(done), days(180)).await;
        let in_active = reading(&db, red, Some(active), days(20)).await;
        let old_idle = reading(&db, red, None, days(100)).await;
        let older_idle = reading(&db, red, None, days(300)).await;
        let recent_idle = reading(&db, green, None, days(5)).await;
        // Unlinked but inside a brew's window: kept by the window guard.
        let unlinked_in_completed_window = reading(&db, red, None, days(170)).await;
        let unlinked_in_active_window = reading(&db, red, None, days(35)).await;
        // Linked to a brew but outside every window (e.g. the brew's dates were
        // edited later): linked, so kept.
        let linked_old = reading(&db, green, Some(done), days(400)).await;

        // User B: their own brew history and idle data. A's brew windows on Red
        // must not protect B's Blue readings.
        let b_done = brew(&db, b, blue, "Archived", days(250), Some(days(210))).await;
        let b_in_brew = reading(&db, blue, Some(b_done), days(230)).await;
        let b_old_idle = reading(&db, blue, None, days(180)).await;
        let b_recent_idle = reading(&db, blue, None, days(1)).await;

        // A device with a wrong clock: recorded long ago, but just received.
        let wrong_clock = Uuid::new_v4();
        readings::ActiveModel {
            id: Set(wrong_clock),
            brew_id: Set(None),
            scanner_id: Set(None),
            hydrometer_id: Set(green),
            temperature_f: Set(68.0),
            gravity: Set(1.0),
            rssi: Set(None),
            recorded_at: Set(days(500).into()),
            created_at: Set(now.into()),
        }
        .insert(&db)
        .await
        .unwrap();

        let deleted = purge_unlinked_readings(&db, 90).await.unwrap();
        assert_eq!(deleted, 3, "old_idle, older_idle and b_old_idle");

        for gone in [old_idle, older_idle, b_old_idle] {
            assert!(
                !exists(&db, gone).await,
                "old unlinked reading should be deleted"
            );
        }
        for kept in [
            in_completed,
            in_active,
            recent_idle,
            unlinked_in_completed_window,
            unlinked_in_active_window,
            linked_old,
            b_in_brew,
            b_recent_idle,
            wrong_clock,
        ] {
            assert!(exists(&db, kept).await, "reading {kept} must be kept");
        }

        // A second run is a no-op.
        assert_eq!(purge_unlinked_readings(&db, 90).await.unwrap(), 0);

        let _ = admin
            .execute_unprepared(&format!("DROP SCHEMA {schema} CASCADE"))
            .await;
    }

    #[rocket::async_test]
    async fn deletes_in_bounded_batches() {
        let Db { db, admin, schema } = fresh_schema().await;
        let a = user(&db, "user-a").await;
        let red = hydrometer(&db, a, "Red").await;
        let old = Utc::now() - chrono::Duration::days(100);
        for _ in 0..7 {
            reading(&db, red, None, old).await;
        }
        let cutoff = Utc::now() - chrono::Duration::days(30);
        assert_eq!(delete_unlinked_batch(&db, cutoff, 3).await.unwrap(), 3);
        assert_eq!(delete_unlinked_batch(&db, cutoff, 3).await.unwrap(), 3);
        assert_eq!(delete_unlinked_batch(&db, cutoff, 3).await.unwrap(), 1);
        assert_eq!(readings::Entity::find().count(&db).await.unwrap(), 0);

        let _ = admin
            .execute_unprepared(&format!("DROP SCHEMA {schema} CASCADE"))
            .await;
    }
}
