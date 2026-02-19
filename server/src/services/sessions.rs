use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter,
};
use uuid::Uuid;

use crate::models::entities::user_sessions::{self, ActiveModel, Entity as UserSession};

pub struct CreateSessionParams {
    pub user_sub: String,
    pub email: String,
    pub name: String,
    pub id_token_hash: String,
    pub expires_in_secs: i64,
}

pub async fn create_session(
    db: &DatabaseConnection,
    params: CreateSessionParams,
) -> Result<user_sessions::Model, DbErr> {
    let now = Utc::now().fixed_offset();
    let expires_at = (Utc::now() + Duration::seconds(params.expires_in_secs)).fixed_offset();

    let model = ActiveModel {
        id: Set(Uuid::new_v4()),
        user_sub: Set(params.user_sub),
        email: Set(params.email),
        name: Set(params.name),
        id_token_hash: Set(params.id_token_hash),
        created_at: Set(now),
        expires_at: Set(expires_at),
        last_seen_at: Set(now),
    };

    model.insert(db).await
}

pub async fn get_session_by_id(
    db: &DatabaseConnection,
    id: Uuid,
) -> Result<Option<user_sessions::Model>, DbErr> {
    UserSession::find_by_id(id).one(db).await
}

pub async fn touch_session(db: &DatabaseConnection, id: Uuid) -> Result<(), DbErr> {
    let session = UserSession::find_by_id(id).one(db).await?;
    if let Some(session) = session {
        let mut active: ActiveModel = session.into();
        active.last_seen_at = Set(Utc::now().fixed_offset());
        active.update(db).await?;
    }
    Ok(())
}

pub async fn delete_session(db: &DatabaseConnection, id: Uuid) -> Result<(), DbErr> {
    UserSession::delete_by_id(id).exec(db).await?;
    Ok(())
}

pub async fn delete_expired_sessions(db: &DatabaseConnection) -> Result<u64, DbErr> {
    let now = Utc::now().fixed_offset();
    let result = UserSession::delete_many()
        .filter(user_sessions::Column::ExpiresAt.lt(now))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_session_params_fields() {
        let params = CreateSessionParams {
            user_sub: "sub|123".to_string(),
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            id_token_hash: "abc123".to_string(),
            expires_in_secs: 86400,
        };
        assert_eq!(params.user_sub, "sub|123");
        assert_eq!(params.email, "test@example.com");
        assert_eq!(params.name, "Test User");
        assert_eq!(params.expires_in_secs, 86400);
    }

    #[test]
    fn expires_at_is_future() {
        let expires_in_secs: i64 = 86400;
        let now = Utc::now();
        let expires_at = now + Duration::seconds(expires_in_secs);
        assert!(expires_at > now, "expires_at should be in the future");
    }

    #[test]
    fn zero_expiry_is_not_future() {
        let expires_in_secs: i64 = 0;
        let now = Utc::now();
        let expires_at = now + Duration::seconds(expires_in_secs);
        assert!(
            expires_at <= now + Duration::milliseconds(10),
            "zero expiry should be approximately now"
        );
    }
}
