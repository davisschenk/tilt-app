//! ESP32 scanners: health heartbeats and the link between readings and the
//! device that uploaded them.

use sea_orm::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::models::entities::scanners::{self, ActiveModel, Column, Entity as Scanner};
use crate::services::{WriteError, limits};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Heartbeat {
    pub device_id: String,
    pub firmware_version: String,
    pub uptime_seconds: u64,
    pub wifi_rssi: Option<i16>,
    pub scan_interval_seconds: u32,
    pub buffered_readings: u32,
    pub scan_ok: bool,
    pub upload_ok: bool,
}

impl Heartbeat {
    /// Reject malformed heartbeats before touching the database.
    pub fn is_valid(&self) -> bool {
        !self.device_id.is_empty()
            && self.device_id.len() <= 64
            && self
                .device_id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-')
            && self.firmware_version.len() <= 64
            && (1..=86_400).contains(&self.scan_interval_seconds)
    }
}

pub async fn find_by_device(
    db: &DatabaseConnection,
    owner: Uuid,
    device_id: &str,
) -> Result<Option<scanners::Model>, DbErr> {
    Scanner::find()
        .filter(Column::OwnerId.eq(owner))
        .filter(Column::DeviceId.eq(device_id))
        .one(db)
        .await
}

pub async fn find_all(db: &DatabaseConnection, owner: Uuid) -> Result<Vec<scanners::Model>, DbErr> {
    Scanner::find()
        .filter(Column::OwnerId.eq(owner))
        .order_by_desc(Column::LastSeenAt)
        .all(db)
        .await
}

/// The owner's scanners as the API returns them: the last heartbeat plus the
/// row id and when it was received.
pub async fn list(db: &DatabaseConnection, owner: Uuid) -> Result<Vec<Value>, DbErr> {
    Ok(find_all(db, owner)
        .await?
        .into_iter()
        .map(|s| {
            let mut v = s.payload;
            v["id"] = json!(s.id);
            v["lastSeenAt"] = json!(s.last_seen_at);
            v
        })
        .collect())
}

/// Record a heartbeat, registering the device on first contact. The caller
/// must have validated `heartbeat` already.
pub async fn record_heartbeat(
    db: &DatabaseConnection,
    owner: Uuid,
    heartbeat: &Heartbeat,
) -> Result<(), WriteError> {
    let exists = find_by_device(db, owner, &heartbeat.device_id)
        .await?
        .is_some();
    if !exists {
        let count = Scanner::find()
            .filter(Column::OwnerId.eq(owner))
            .count(db)
            .await?;
        if count >= limits::MAX_SCANNERS {
            return Err(WriteError::LimitReached("scanners"));
        }
    }
    Scanner::insert(ActiveModel {
        id: Set(Uuid::new_v4()),
        owner_id: Set(owner),
        device_id: Set(heartbeat.device_id.clone()),
        payload: Set(json!(heartbeat)),
        last_seen_at: Set(chrono::Utc::now().into()),
    })
    .on_conflict(
        sea_query::OnConflict::columns([Column::OwnerId, Column::DeviceId])
            .update_columns([Column::Payload, Column::LastSeenAt])
            .to_owned(),
    )
    .exec(db)
    .await?;
    Ok(())
}
