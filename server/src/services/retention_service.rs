//! Opt-in retention for readings that are not part of any brew.
//!
//! A reading belongs to a brew through `readings.brew_id`, which ingest sets to
//! the hydrometer's brew whose status is exactly `Active` when the reading
//! arrives. Every brew view (chart, analytics, report, `GET /readings?brew_id=`)
//! selects by that column. Readings with `brew_id IS NULL` are what a hydrometer
//! reports while idle between batches, and they are the only rows this module
//! ever deletes.
//!
//! An unlinked reading is still kept if its hydrometer has any brew whose window
//! `[start_date, end_date-or-now]` covers its `recorded_at`. Because `brew_id`
//! is only set while a brew is `Active`, and users can change a brew's status
//! and end date afterwards (pause it, reopen it, move the end), a reading inside
//! a brew's window can be unlinked. This check errs towards keeping data.
//!
//! Age is measured by both the device-supplied `recorded_at` and the server's
//! `created_at`, so a device with a wrong clock cannot make a fresh reading look
//! old.

use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement};
use std::time::Duration;

/// Rows removed per statement, so one run never holds long locks on `readings`.
pub const BATCH_SIZE: u64 = 5_000;
/// Pause between batches, to let ingest and queries interleave.
const BATCH_PAUSE: Duration = Duration::from_millis(100);

/// The result of one batch: rows deleted, and the latest `recorded_at` among
/// them (where the next batch can resume).
pub struct Batch {
    pub deleted: u64,
    pub last_recorded_at: Option<DateTime<Utc>>,
}

/// Delete up to `limit` unlinked readings older than `cutoff`, oldest first,
/// starting at `recorded_at >= from`. Fewer than `limit` deleted means there
/// are none left.
pub async fn delete_unlinked_batch(
    db: &impl ConnectionTrait,
    cutoff: DateTime<Utc>,
    from: DateTime<Utc>,
    limit: u64,
) -> Result<Batch, DbErr> {
    // Walks idx_readings_brew_recorded (brew_id, recorded_at) from `from`, so a
    // large backlog does not re-scan the index entries of rows already deleted
    // in this run; the NOT EXISTS probe uses idx_brews_hydrometer_id.
    let sql = r#"
        WITH doomed AS (
            SELECT r.id FROM readings r
            WHERE r.brew_id IS NULL
              AND r.recorded_at >= $2
              AND r.recorded_at < $1
              AND r.created_at < $1
              AND NOT EXISTS (
                  SELECT 1 FROM brews b
                  WHERE b.hydrometer_id = r.hydrometer_id
                    AND r.recorded_at >= COALESCE(b.start_date, b.created_at)
                    AND r.recorded_at <= COALESCE(b.end_date, now())
              )
            ORDER BY r.recorded_at
            LIMIT $3
        ), gone AS (
            DELETE FROM readings WHERE id IN (SELECT id FROM doomed)
            RETURNING recorded_at
        )
        SELECT count(*)::bigint AS deleted, max(recorded_at) AS last FROM gone
    "#;
    let row = db
        .query_one_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            [cutoff.into(), from.into(), (limit as i64).into()],
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("retention batch returned no row".into()))?;
    let deleted: i64 = row.try_get("", "deleted")?;
    let last: Option<DateTime<FixedOffset>> = row.try_get("", "last")?;
    Ok(Batch {
        deleted: deleted as u64,
        last_recorded_at: last.map(|t| t.with_timezone(&Utc)),
    })
}

/// Delete every unlinked reading older than `days`, in bounded batches.
pub async fn purge_unlinked_readings(db: &DatabaseConnection, days: u32) -> Result<u64, DbErr> {
    let cutoff = Utc::now() - chrono::Duration::days(i64::from(days));
    // Year 1: before any real reading, and inside Postgres' timestamptz range.
    let mut from = DateTime::<Utc>::from_timestamp(-62_135_596_800, 0).expect("valid timestamp");
    let mut total = 0;
    loop {
        let batch = delete_unlinked_batch(db, cutoff, from, BATCH_SIZE).await?;
        total += batch.deleted;
        match batch.last_recorded_at {
            // `>=` on resume: rows sharing the last timestamp may remain.
            Some(last) if batch.deleted >= BATCH_SIZE => from = last,
            _ => return Ok(total),
        }
        tokio::time::sleep(BATCH_PAUSE).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::entities::{brews, hydrometers, readings};
    use crate::services::user_service;
    use crate::test_db::TestDb;
    use sea_orm::{ActiveModelTrait, EntityTrait, PaginatorTrait, Set};
    use uuid::Uuid;

    async fn user(db: &DatabaseConnection, sub: &str) -> Uuid {
        user_service::upsert(db, sub, &format!("{sub}@example.com"), sub)
            .await
            .unwrap()
            .id
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

    /// A brew on `hydrometer`, Active while it has no end date.
    async fn brew(
        db: &DatabaseConnection,
        owner: Uuid,
        hydrometer: Uuid,
        start: DateTime<Utc>,
        end: Option<DateTime<Utc>>,
    ) -> Uuid {
        let id = Uuid::new_v4();
        brews::ActiveModel {
            id: Set(id),
            owner_id: Set(owner),
            name: Set("brew".into()),
            style: Set(None),
            og: Set(None),
            fg: Set(None),
            target_fg: Set(None),
            status: Set(if end.is_some() { "Completed" } else { "Active" }.into()),
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

    async fn reading_at(
        db: &DatabaseConnection,
        hydrometer: Uuid,
        brew: Option<Uuid>,
        recorded: DateTime<Utc>,
        created: DateTime<Utc>,
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
            recorded_at: Set(recorded.into()),
            created_at: Set(created.into()),
        }
        .insert(db)
        .await
        .unwrap();
        id
    }

    /// A reading recorded and received at `at`.
    async fn reading(
        db: &DatabaseConnection,
        hydrometer: Uuid,
        brew: Option<Uuid>,
        at: DateTime<Utc>,
    ) -> Uuid {
        reading_at(db, hydrometer, brew, at, at).await
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
        let t = TestDb::new().await;
        let db = &t.db;
        let now = Utc::now();
        let days = |n: i64| now - chrono::Duration::days(n);

        let a = user(db, "user-a").await;
        let b = user(db, "user-b").await;
        let red = hydrometer(db, a, "Red").await;
        let green = hydrometer(db, a, "Green").await;
        let blue = hydrometer(db, b, "Blue").await;

        // Red: a completed brew 200..150 days ago, then idle, then an active
        // brew since 40 days ago.
        let done = brew(db, a, red, days(200), Some(days(150))).await;
        let active = brew(db, a, red, days(40), None).await;
        let in_completed = reading(db, red, Some(done), days(180)).await;
        let in_active = reading(db, red, Some(active), days(20)).await;
        let old_idle = reading(db, red, None, days(100)).await;
        let older_idle = reading(db, red, None, days(300)).await;
        let recent_idle = reading(db, green, None, days(5)).await;
        // Unlinked but inside a brew's window: kept by the window check.
        let unlinked_in_completed_window = reading(db, red, None, days(170)).await;
        let unlinked_in_active_window = reading(db, red, None, days(35)).await;
        // Linked to a brew but outside every window (e.g. the brew's dates were
        // edited later): linked, so kept.
        let linked_old = reading(db, green, Some(done), days(400)).await;
        // A device with a wrong clock: recorded long ago, but just received.
        let wrong_clock = reading_at(db, green, None, days(500), now).await;

        // User B: their own brew history and idle data. A's brew windows on Red
        // must not protect B's Blue readings.
        let b_done = brew(db, b, blue, days(250), Some(days(210))).await;
        let b_in_brew = reading(db, blue, Some(b_done), days(230)).await;
        let b_old_idle = reading(db, blue, None, days(180)).await;
        let b_recent_idle = reading(db, blue, None, days(1)).await;

        let deleted = purge_unlinked_readings(db, 90).await.unwrap();
        assert_eq!(deleted, 3, "old_idle, older_idle and b_old_idle");

        for gone in [old_idle, older_idle, b_old_idle] {
            assert!(
                !exists(db, gone).await,
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
            wrong_clock,
            b_in_brew,
            b_recent_idle,
        ] {
            assert!(exists(db, kept).await, "reading {kept} must be kept");
        }

        // A second run is a no-op.
        assert_eq!(purge_unlinked_readings(db, 90).await.unwrap(), 0);

        t.cleanup().await;
    }

    #[rocket::async_test]
    async fn deletes_in_bounded_batches_resuming_from_the_cursor() {
        let t = TestDb::new().await;
        let db = &t.db;
        let a = user(db, "user-a").await;
        let red = hydrometer(db, a, "Red").await;
        let base = Utc::now() - chrono::Duration::days(100);
        for i in 0..7 {
            reading(db, red, None, base + chrono::Duration::minutes(i)).await;
        }
        let cutoff = Utc::now() - chrono::Duration::days(30);
        let mut from = base - chrono::Duration::days(1);
        let mut sizes = Vec::new();
        loop {
            let batch = delete_unlinked_batch(db, cutoff, from, 3).await.unwrap();
            sizes.push(batch.deleted);
            match batch.last_recorded_at {
                Some(last) if batch.deleted == 3 => {
                    assert!(last >= from, "cursor only moves forward");
                    from = last;
                }
                _ => break,
            }
        }
        assert_eq!(sizes, vec![3, 3, 1]);
        assert_eq!(readings::Entity::find().count(db).await.unwrap(), 0);

        t.cleanup().await;
    }
}
