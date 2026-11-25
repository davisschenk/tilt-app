//! Account export and deletion (data portability and the right to erasure).

use sea_orm::sea_query::Query;
use sea_orm::*;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::models::entities::{
    alert_rules, alert_targets, api_keys, brew_events, brews, event_attachments, hydrometers,
    readings, user_sessions, users,
};
use crate::services::{preferences_service, scanner_service};

/// Serialise a row and drop fields that must never leave the server.
fn row<M: serde::Serialize>(model: &M, strip: &[&str]) -> Value {
    let mut v = serde_json::to_value(model).unwrap_or(Value::Null);
    if let Some(obj) = v.as_object_mut() {
        for k in strip {
            obj.remove(*k);
        }
    }
    v
}

fn owned_hydrometer_ids(owner: Uuid) -> sea_orm::sea_query::SelectStatement {
    Query::select()
        .column(hydrometers::Column::Id)
        .from(hydrometers::Entity)
        .and_where(hydrometers::Column::OwnerId.eq(owner))
        .to_owned()
}

fn owned_brew_ids(owner: Uuid) -> sea_orm::sea_query::SelectStatement {
    Query::select()
        .column(brews::Column::Id)
        .from(brews::Entity)
        .and_where(brews::Column::OwnerId.eq(owner))
        .to_owned()
}

/// Everything the user owns, as one JSON document. Secrets are left out: API key
/// hashes and webhook secret headers are not exported. Attachments are listed
/// (name, type, size, download path) but their bytes are not inlined.
pub async fn export(db: &DatabaseConnection, user: &users::Model) -> Result<Value, DbErr> {
    let owner = user.id;

    let hydros = hydrometers::Entity::find()
        .filter(hydrometers::Column::OwnerId.eq(owner))
        .all(db)
        .await?;
    let brews_ = brews::Entity::find()
        .filter(brews::Column::OwnerId.eq(owner))
        .all(db)
        .await?;
    let events = brew_events::Entity::find()
        .filter(brew_events::Column::BrewId.in_subquery(owned_brew_ids(owner)))
        .all(db)
        .await?;
    let attachments = event_attachments::Entity::find()
        .filter(
            event_attachments::Column::EventId.in_subquery(
                Query::select()
                    .column(brew_events::Column::Id)
                    .from(brew_events::Entity)
                    .and_where(brew_events::Column::BrewId.in_subquery(owned_brew_ids(owner)))
                    .to_owned(),
            ),
        )
        .all(db)
        .await?;
    let targets = alert_targets::Entity::find()
        .filter(alert_targets::Column::OwnerId.eq(owner))
        .all(db)
        .await?;
    let rules = alert_rules::Entity::find()
        .filter(
            alert_rules::Column::AlertTargetId.in_subquery(
                Query::select()
                    .column(alert_targets::Column::Id)
                    .from(alert_targets::Entity)
                    .and_where(alert_targets::Column::OwnerId.eq(owner))
                    .to_owned(),
            ),
        )
        .all(db)
        .await?;
    let keys = api_keys::Entity::find()
        .filter(api_keys::Column::OwnerId.eq(owner))
        .all(db)
        .await?;
    let reading_rows = readings::Entity::find()
        .filter(readings::Column::HydrometerId.in_subquery(owned_hydrometer_ids(owner)))
        .order_by_asc(readings::Column::RecordedAt)
        .all(db)
        .await?;

    let preferences = preferences_service::find_raw(db, owner).await?;
    let scanners = scanner_service::find_all(db, owner).await?;

    Ok(json!({
        "exportedAt": chrono::Utc::now(),
        "preferences": preferences.map(|r| r.payload),
        "scanners": scanners.into_iter().map(|r| json!({
            "deviceId": r.device_id,
            "lastSeenAt": r.last_seen_at,
            "health": r.payload,
        })).collect::<Vec<_>>(),
        "account": { "id": user.id, "email": user.email, "name": user.name, "createdAt": user.created_at },
        "hydrometers": hydros.iter().map(|m| row(m, &[])).collect::<Vec<_>>(),
        "brews": brews_.iter().map(|m| row(m, &[])).collect::<Vec<_>>(),
        "brewEvents": events.iter().map(|m| row(m, &[])).collect::<Vec<_>>(),
        "attachments": attachments.iter().map(|m| {
            let mut v = row(m, &["storage_path"]);
            v["downloadPath"] = json!(format!("/api/v1/attachments/{}/file", m.id));
            v
        }).collect::<Vec<_>>(),
        "alertTargets": targets.iter().map(|m| {
            let mut v = row(m, &["secret_header"]);
            v["hasSecretHeader"] = json!(m.secret_header.is_some());
            v
        }).collect::<Vec<_>>(),
        "alertRules": rules.iter().map(|m| row(m, &[])).collect::<Vec<_>>(),
        "apiKeys": keys.iter().map(|m| row(m, &["key_hash"])).collect::<Vec<_>>(),
        "readings": reading_rows.iter().map(|m| row(m, &[])).collect::<Vec<_>>(),
    }))
}

/// Permanently delete a user and everything they own, in one transaction.
/// Returns the ids of events that had attachments so the caller can remove the
/// files from disk after the database commit.
///
/// Deleted explicitly, in dependency order, because `readings` and `brews`
/// restrict deletion of a hydrometer, so a cascade from the user row alone would
/// be refused.
pub async fn delete(db: &DatabaseConnection, user: &users::Model) -> Result<Vec<Uuid>, DbErr> {
    let owner = user.id;
    let txn = db.begin().await?;

    let events_with_files: Vec<Uuid> = event_attachments::Entity::find()
        .select_only()
        .column(event_attachments::Column::EventId)
        .distinct()
        .filter(
            event_attachments::Column::EventId.in_subquery(
                Query::select()
                    .column(brew_events::Column::Id)
                    .from(brew_events::Entity)
                    .and_where(brew_events::Column::BrewId.in_subquery(owned_brew_ids(owner)))
                    .to_owned(),
            ),
        )
        .into_tuple()
        .all(&txn)
        .await?;

    // Targets first (their rules cascade), then brews (events + attachments cascade).
    alert_targets::Entity::delete_many()
        .filter(alert_targets::Column::OwnerId.eq(owner))
        .exec(&txn)
        .await?;
    brews::Entity::delete_many()
        .filter(brews::Column::OwnerId.eq(owner))
        .exec(&txn)
        .await?;
    readings::Entity::delete_many()
        .filter(readings::Column::HydrometerId.in_subquery(owned_hydrometer_ids(owner)))
        .exec(&txn)
        .await?;
    hydrometers::Entity::delete_many()
        .filter(hydrometers::Column::OwnerId.eq(owner))
        .exec(&txn)
        .await?;
    api_keys::Entity::delete_many()
        .filter(api_keys::Column::OwnerId.eq(owner))
        .exec(&txn)
        .await?;
    user_sessions::Entity::delete_many()
        .filter(user_sessions::Column::UserSub.eq(user.oidc_sub.clone()))
        .exec(&txn)
        .await?;
    users::Entity::delete_by_id(owner).exec(&txn).await?;

    txn.commit().await?;
    Ok(events_with_files)
}
