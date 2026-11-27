use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::models::entities::api_keys::{self, ActiveModel, Entity as ApiKey};

#[derive(Debug)]
pub enum ApiKeyError {
    Expired,
    Invalid,
    LimitReached,
    #[allow(dead_code)]
    Db(DbErr),
}

impl From<DbErr> for ApiKeyError {
    fn from(e: DbErr) -> Self {
        ApiKeyError::Db(e)
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyCreated {
    pub id: Uuid,
    pub name: String,
    pub prefix: String,
    pub key: String,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
    pub message: &'static str,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeySummary {
    pub id: Uuid,
    pub name: String,
    pub prefix: String,
    pub created_by: String,
    pub last_used_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub expires_at: Option<chrono::DateTime<chrono::FixedOffset>>,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
}

/// Lowercase hex SHA-256: the format of `api_keys.key_hash` and
/// `user_sessions.id_token_hash`. It must never change, or existing API keys
/// stop verifying.
pub fn sha256_hex(input: &str) -> String {
    hex::encode(Sha256::digest(input.as_bytes()))
}

pub fn generate_api_key() -> (String, String, String) {
    let raw_bytes: [u8; 32] = rand::random();
    let raw = hex::encode(raw_bytes);
    let hash = sha256_hex(&raw);
    let prefix = raw[..8].to_string();
    (raw, hash, prefix)
}

pub async fn create_api_key(
    db: &DatabaseConnection,
    owner: Uuid,
    name: String,
    created_by: String,
    expires_at: Option<chrono::DateTime<chrono::FixedOffset>>,
) -> Result<ApiKeyCreated, ApiKeyError> {
    if ApiKey::find()
        .filter(api_keys::Column::OwnerId.eq(owner))
        .count(db)
        .await?
        >= crate::services::limits::MAX_API_KEYS
    {
        return Err(ApiKeyError::LimitReached);
    }
    let (raw, hash, prefix) = generate_api_key();
    let now = Utc::now().fixed_offset();

    let model = ActiveModel {
        id: Set(Uuid::new_v4()),
        owner_id: Set(owner),
        name: Set(name.clone()),
        key_hash: Set(hash),
        prefix: Set(prefix.clone()),
        created_by: Set(created_by),
        last_used_at: Set(None),
        expires_at: Set(expires_at),
        created_at: Set(now),
    };

    let inserted = model.insert(db).await?;

    Ok(ApiKeyCreated {
        id: inserted.id,
        name: inserted.name,
        prefix,
        key: raw,
        created_at: inserted.created_at,
        message: "Store this key securely — it will not be shown again.",
    })
}

pub async fn list_api_keys(
    db: &DatabaseConnection,
    owner: Uuid,
) -> Result<Vec<ApiKeySummary>, DbErr> {
    let keys = ApiKey::find()
        .filter(api_keys::Column::OwnerId.eq(owner))
        .all(db)
        .await?;

    Ok(keys
        .into_iter()
        .map(|k| ApiKeySummary {
            id: k.id,
            name: k.name,
            prefix: k.prefix,
            created_by: k.created_by,
            last_used_at: k.last_used_at,
            expires_at: k.expires_at,
            created_at: k.created_at,
        })
        .collect())
}

/// Delete one of `owner`'s keys. Deleting a key that does not exist, or that
/// belongs to someone else, is a silent no-op so ids cannot be probed.
pub async fn delete_api_key(
    db: &DatabaseConnection,
    id: Uuid,
    owner: Uuid,
) -> Result<(), ApiKeyError> {
    ApiKey::delete_many()
        .filter(api_keys::Column::Id.eq(id))
        .filter(api_keys::Column::OwnerId.eq(owner))
        .exec(db)
        .await
        .map_err(ApiKeyError::Db)?;
    Ok(())
}

pub async fn validate_api_key(
    db: &DatabaseConnection,
    raw_key: &str,
) -> Result<api_keys::Model, ApiKeyError> {
    let hash = sha256_hex(raw_key);

    let key = ApiKey::find()
        .filter(api_keys::Column::KeyHash.eq(&hash))
        .one(db)
        .await
        .map_err(ApiKeyError::Db)?
        .ok_or(ApiKeyError::Invalid)?;

    if let Some(expires_at) = key.expires_at
        && expires_at < Utc::now().fixed_offset()
    {
        return Err(ApiKeyError::Expired);
    }

    let mut active: api_keys::ActiveModel = key.clone().into();
    active.last_used_at = Set(Some(Utc::now().fixed_offset()));
    active.update(db).await.map_err(ApiKeyError::Db)?;

    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_api_key_length() {
        let (raw, hash, prefix) = generate_api_key();
        assert_eq!(raw.len(), 64, "raw key should be 64 hex chars (32 bytes)");
        assert_eq!(hash.len(), 64, "sha256 hash should be 64 hex chars");
        assert_eq!(prefix.len(), 8, "prefix should be first 8 chars of raw key");
        assert!(raw.starts_with(&prefix));
    }

    #[test]
    fn generate_api_key_uniqueness() {
        let (raw1, hash1, _) = generate_api_key();
        let (raw2, hash2, _) = generate_api_key();
        assert_ne!(raw1, raw2, "two generated keys should differ");
        assert_ne!(hash1, hash2, "two generated hashes should differ");
    }

    #[test]
    fn sha256_hex_matches_known_vectors() {
        // Pinned so a hashing-crate upgrade can never silently change stored hashes.
        assert_eq!(
            sha256_hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(&"deadbeef".repeat(8)),
            "247d08f3e13938b244f5ecd8966f1778e5e72b175820f46ba86c9c039272affa"
        );
    }

    #[test]
    fn generate_api_key_hash_differs_from_raw() {
        let (raw, hash, _) = generate_api_key();
        assert_ne!(raw, hash, "hash should differ from raw key");
    }

    #[test]
    fn generate_api_key_is_hex() {
        let (raw, hash, prefix) = generate_api_key();
        assert!(raw.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(prefix.chars().all(|c| c.is_ascii_hexdigit()));
    }
}
