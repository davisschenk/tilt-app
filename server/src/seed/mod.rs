//! Deterministic dev/test data seeder. Used by `just seed` and by integration
//! tests. Runs against the live database via SeaORM entities — schema changes
//! that break the seed break the build.

mod curves;

use anyhow::Result;
use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait};

use crate::models::entities::hydrometers;

/// Run the minimal seed profile. No-op if hydrometers table already has rows
/// (call with `force=true` to wipe and reseed).
pub async fn seed_minimal(db: &DatabaseConnection, force: bool) -> Result<SeedReport> {
    if !force {
        let existing = hydrometers::Entity::find().count(db).await?;
        if existing > 0 {
            return Ok(SeedReport::skipped(existing));
        }
    } else {
        wipe(db).await?;
    }

    let mut report = SeedReport::default();
    seed_hydrometers(db, &mut report).await?;
    seed_brews_and_readings(db, &mut report).await?;
    seed_events(db, &mut report).await?;
    seed_alerts(db, &mut report).await?;
    Ok(report)
}

#[derive(Default, Debug)]
pub struct SeedReport {
    pub hydrometers: u64,
    pub brews: u64,
    pub readings: u64,
    pub events: u64,
    pub alert_targets: u64,
    pub alert_rules: u64,
    pub skipped_existing: Option<u64>,
}

impl SeedReport {
    fn skipped(existing: u64) -> Self {
        Self {
            skipped_existing: Some(existing),
            ..Default::default()
        }
    }
}

pub mod ids {
    //! Stable seed UUIDs so tests can assert against them.
    use uuid::Uuid;

    pub const HYDROMETER_RED: Uuid = Uuid::from_bytes([
        0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x11,
        0x11,
    ]);
    pub const HYDROMETER_BLACK: Uuid = Uuid::from_bytes([
        0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22, 0x22,
        0x22,
    ]);
    pub const HYDROMETER_GREEN: Uuid = Uuid::from_bytes([
        0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33, 0x33,
        0x33,
    ]);
    pub const BREW_ACTIVE: Uuid = Uuid::from_bytes([
        0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa,
        0xaa,
    ]);
    pub const BREW_COMPLETED: Uuid = Uuid::from_bytes([
        0xbb, 0xbb, 0xbb, 0xbb, 0xbb, 0xbb, 0xbb, 0xbb, 0xbb, 0xbb, 0xbb, 0xbb, 0xbb, 0xbb, 0xbb,
        0xbb,
    ]);
    pub const ALERT_TARGET: Uuid = Uuid::from_bytes([
        0xcc, 0xcc, 0xcc, 0xcc, 0xcc, 0xcc, 0xcc, 0xcc, 0xcc, 0xcc, 0xcc, 0xcc, 0xcc, 0xcc, 0xcc,
        0xcc,
    ]);
    pub const ALERT_RULE: Uuid = Uuid::from_bytes([
        0xdd, 0xdd, 0xdd, 0xdd, 0xdd, 0xdd, 0xdd, 0xdd, 0xdd, 0xdd, 0xdd, 0xdd, 0xdd, 0xdd, 0xdd,
        0xdd,
    ]);
}

async fn seed_hydrometers(db: &DatabaseConnection, r: &mut SeedReport) -> Result<()> {
    use crate::models::entities::hydrometers;
    use sea_orm::Set;

    let now = chrono::Utc::now().fixed_offset();

    let rows = vec![
        hydrometers::ActiveModel {
            id: Set(ids::HYDROMETER_RED),
            color: Set("Red".into()),
            name: Set(Some("Mash Tun".into())),
            temp_offset_f: Set(0.0),
            gravity_offset: Set(0.0),
            created_at: Set(now),
            is_disabled: Set(false),
        },
        hydrometers::ActiveModel {
            id: Set(ids::HYDROMETER_BLACK),
            color: Set("Black".into()),
            name: Set(Some("Conical".into())),
            temp_offset_f: Set(0.0),
            gravity_offset: Set(0.0),
            created_at: Set(now),
            is_disabled: Set(true),
        },
        hydrometers::ActiveModel {
            id: Set(ids::HYDROMETER_GREEN),
            color: Set("Green".into()),
            name: Set(None),
            temp_offset_f: Set(0.0),
            gravity_offset: Set(0.0),
            created_at: Set(now),
            is_disabled: Set(false),
        },
    ];

    let n = rows.len() as u64;
    hydrometers::Entity::insert_many(rows).exec(db).await?;
    r.hydrometers = n;
    Ok(())
}

async fn seed_brews_and_readings(db: &DatabaseConnection, r: &mut SeedReport) -> Result<()> {
    use crate::models::entities::{brews, readings};
    use chrono::{Duration, Utc};
    use sea_orm::Set;
    use uuid::Uuid;

    let now = Utc::now().fixed_offset();

    // Active brew: West Coast IPA on Red, started 24h ago, OG 1.062 → dropping toward 1.040.
    let active_start = now - Duration::hours(24);
    let active = brews::ActiveModel {
        id: Set(ids::BREW_ACTIVE),
        name: Set("West Coast IPA".into()),
        style: Set(Some("American IPA".into())),
        og: Set(Some(1.062)),
        fg: Set(None),
        target_fg: Set(Some(1.012)),
        status: Set("Active".into()),
        start_date: Set(Some(active_start)),
        end_date: Set(None),
        notes: Set(Some("Seeded brew — deterministic test data".into())),
        hydrometer_id: Set(ids::HYDROMETER_RED),
        created_at: Set(active_start),
        updated_at: Set(now),
        batch_size_gallons: Set(Some(5.0)),
        yeast_nitrogen_requirement: Set(None),
        pitch_time: Set(Some(active_start)),
        nutrient_protocol: Set(None),
        yeast_strain: Set(Some("US-05".into())),
        nutrient_alert_target_id: Set(None),
    };

    // Completed brew: Imperial Stout on Black, finished 30 days ago.
    let completed_start = now - Duration::days(60);
    let completed_end = now - Duration::days(30);
    let completed = brews::ActiveModel {
        id: Set(ids::BREW_COMPLETED),
        name: Set("Imperial Stout".into()),
        style: Set(Some("Russian Imperial Stout".into())),
        og: Set(Some(1.085)),
        fg: Set(Some(1.018)),
        target_fg: Set(Some(1.020)),
        status: Set("Completed".into()),
        start_date: Set(Some(completed_start)),
        end_date: Set(Some(completed_end)),
        notes: Set(Some("Seeded brew — deterministic test data".into())),
        hydrometer_id: Set(ids::HYDROMETER_BLACK),
        created_at: Set(completed_start),
        updated_at: Set(completed_end),
        batch_size_gallons: Set(Some(5.0)),
        yeast_nitrogen_requirement: Set(None),
        pitch_time: Set(Some(completed_start)),
        nutrient_protocol: Set(None),
        yeast_strain: Set(Some("WLP099".into())),
        nutrient_alert_target_id: Set(None),
    };

    brews::Entity::insert_many([active, completed])
        .exec(db)
        .await?;
    r.brews = 2;

    // Readings: every 5 minutes for 24h on the active brew.
    let mut active_readings = Vec::new();
    let total_minutes = 24.0 * 60.0;
    let mut t = 0.0_f64;
    let mut idx: usize = 0;
    while t <= total_minutes {
        let recorded_at = active_start + Duration::seconds((t * 60.0) as i64);
        active_readings.push(readings::ActiveModel {
            id: Set(Uuid::new_v4()),
            brew_id: Set(Some(ids::BREW_ACTIVE)),
            hydrometer_id: Set(ids::HYDROMETER_RED),
            temperature_f: Set(curves::temperature_at(68.0, 1.5, idx)),
            gravity: Set(curves::gravity_at(1.062, 1.040, total_minutes, t)),
            rssi: Set(Some(-70)),
            recorded_at: Set(recorded_at),
            created_at: Set(recorded_at),
        });
        t += 5.0;
        idx += 1;
    }

    // Readings: every 30 minutes for 14 days on the completed brew.
    let mut completed_readings = Vec::new();
    let completed_total_minutes = 14.0 * 24.0 * 60.0;
    let mut t = 0.0_f64;
    let mut idx: usize = 0;
    while t <= completed_total_minutes {
        let recorded_at = completed_start + Duration::seconds((t * 60.0) as i64);
        completed_readings.push(readings::ActiveModel {
            id: Set(Uuid::new_v4()),
            brew_id: Set(Some(ids::BREW_COMPLETED)),
            hydrometer_id: Set(ids::HYDROMETER_BLACK),
            temperature_f: Set(curves::temperature_at(64.0, 2.0, idx)),
            gravity: Set(curves::gravity_at(1.085, 1.018, completed_total_minutes, t)),
            rssi: Set(Some(-75)),
            recorded_at: Set(recorded_at),
            created_at: Set(recorded_at),
        });
        t += 30.0;
        idx += 1;
    }

    let active_n = active_readings.len() as u64;
    let completed_n = completed_readings.len() as u64;
    // Insert in chunks — Postgres has a parameter cap; 500/chunk is safe.
    for chunk in active_readings.chunks(500) {
        readings::Entity::insert_many(chunk.to_vec())
            .exec(db)
            .await?;
    }
    for chunk in completed_readings.chunks(500) {
        readings::Entity::insert_many(chunk.to_vec())
            .exec(db)
            .await?;
    }
    r.readings = active_n + completed_n;

    Ok(())
}

async fn seed_events(db: &DatabaseConnection, r: &mut SeedReport) -> Result<()> {
    use crate::models::entities::brew_events;
    use chrono::{Duration, Utc};
    use sea_orm::Set;
    use uuid::Uuid;

    let now = Utc::now().fixed_offset();
    let active_start = now - Duration::hours(24);

    let events = vec![
        brew_events::ActiveModel {
            id: Set(Uuid::new_v4()),
            brew_id: Set(ids::BREW_ACTIVE),
            event_type: Set("DryHop".into()),
            label: Set("Dry hop addition".into()),
            notes: Set(Some("Citra + Mosaic, 2oz each".into())),
            gravity_at_event: Set(Some(1.052)),
            temp_at_event: Set(Some(67.5)),
            event_time: Set(active_start + Duration::hours(12)),
            created_at: Set(active_start + Duration::hours(12)),
        },
        brew_events::ActiveModel {
            id: Set(Uuid::new_v4()),
            brew_id: Set(ids::BREW_ACTIVE),
            event_type: Set("ColdCrash".into()),
            label: Set("Cold crash started".into()),
            notes: Set(None),
            gravity_at_event: Set(Some(1.043)),
            temp_at_event: Set(Some(35.0)),
            event_time: Set(active_start + Duration::hours(20)),
            created_at: Set(active_start + Duration::hours(20)),
        },
    ];

    let n = events.len() as u64;
    brew_events::Entity::insert_many(events).exec(db).await?;
    r.events = n;
    Ok(())
}

async fn seed_alerts(db: &DatabaseConnection, r: &mut SeedReport) -> Result<()> {
    use crate::models::entities::{alert_rules, alert_targets};
    use chrono::Utc;
    use sea_orm::Set;

    let now = Utc::now().fixed_offset();

    let target = alert_targets::ActiveModel {
        id: Set(ids::ALERT_TARGET),
        name: Set("Local webhook stub".into()),
        url: Set("http://localhost:9999/webhook".into()),
        format: Set("generic".into()),
        secret_header: Set(None),
        enabled: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
    };
    alert_targets::Entity::insert(target).exec(db).await?;
    r.alert_targets = 1;

    let rule = alert_rules::ActiveModel {
        id: Set(ids::ALERT_RULE),
        name: Set("FG threshold reached".into()),
        brew_id: Set(Some(ids::BREW_ACTIVE)),
        hydrometer_id: Set(None),
        metric: Set("gravity".into()),
        operator: Set("lte".into()),
        threshold: Set(1.040),
        alert_target_id: Set(ids::ALERT_TARGET),
        enabled: Set(true),
        cooldown_minutes: Set(60),
        last_triggered_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        window_hours: Set(24),
    };
    alert_rules::Entity::insert(rule).exec(db).await?;
    r.alert_rules = 1;

    Ok(())
}

async fn wipe(db: &DatabaseConnection) -> Result<()> {
    use crate::models::entities::{
        alert_rules, alert_targets, brew_events, brews, event_attachments, hydrometers, readings,
    };
    // Order matters — children before parents.
    event_attachments::Entity::delete_many().exec(db).await?;
    brew_events::Entity::delete_many().exec(db).await?;
    readings::Entity::delete_many().exec(db).await?;
    alert_rules::Entity::delete_many().exec(db).await?;
    alert_targets::Entity::delete_many().exec(db).await?;
    brews::Entity::delete_many().exec(db).await?;
    hydrometers::Entity::delete_many().exec(db).await?;
    Ok(())
}
