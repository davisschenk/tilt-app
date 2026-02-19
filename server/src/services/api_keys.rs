use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::models::entities::api_keys::{self, ActiveModel, Entity as ApiKey};

#[derive(Debug)]
pub enum ApiKeyError {
    Expired,
    Invalid,
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

pub fn generate_api_key() -> (String, String, String) {
    let raw_bytes: [u8; 32] = rand::random();
    let raw = hex::encode(raw_bytes);
    let hash = format!("{:x}", Sha256::digest(raw.as_bytes()));
    let prefix = raw[..8].to_string();
    (raw, hash, prefix)
}

pub async fn create_api_key(
    db: &DatabaseConnection,
    name: String,
    created_by: String,
    expires_at: Option<chrono::DateTime<chrono::FixedOffset>>,
) -> Result<ApiKeyCreated, DbErr> {
    let (raw, hash, prefix) = generate_api_key();
    let now = Utc::now().fixed_offset();

    let model = ActiveModel {
        id: Set(Uuid::new_v4()),
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
    user_sub: &str,
) -> Result<Vec<ApiKeySummary>, DbErr> {
    let keys = ApiKey::find()
        .filter(api_keys::Column::CreatedBy.eq(user_sub))
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

pub async fn delete_api_key(
    db: &DatabaseConnection,
    id: Uuid,
    user_sub: &str,
) -> Result<(), ApiKeyError> {
    let key = ApiKey::find_by_id(id)
        .one(db)
        .await
        .map_err(ApiKeyError::Db)?;

    match key {
        None => Ok(()),
        Some(k) if k.created_by != user_sub => Err(ApiKeyError::Invalid),
        Some(_) => {
            ApiKey::delete_by_id(id)
                .exec(db)
                .await
                .map_err(ApiKeyError::Db)?;
            Ok(())
        }
    }
}

pub async fn validate_api_key(
    db: &DatabaseConnection,
    raw_key: &str,
) -> Result<api_keys::Model, ApiKeyError> {
    let hash = format!("{:x}", Sha256::digest(raw_key.as_bytes()));

    let key = ApiKey::find()
        .filter(api_keys::Column::KeyHash.eq(&hash))
        .one(db)
        .await
        .map_err(ApiKeyError::Db)?
        .ok_or(ApiKeyError::Invalid)?;

    if let Some(expires_at) = key.expires_at {
        if expires_at < Utc::now().fixed_offset() {
            return Err(ApiKeyError::Expired);
        }
    }

    let mut active: api_keys::ActiveModel = key.clone().into();
    active.last_used_at = Set(Some(Utc::now().fixed_offset()));
    active.update(db).await.map_err(ApiKeyError::Db)?;

    Ok(key)
}
