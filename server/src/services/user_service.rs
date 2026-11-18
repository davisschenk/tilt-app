use sea_orm::{
    ActiveValue::Set, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter,
    sea_query::OnConflict,
};
use uuid::Uuid;

use crate::models::entities::users::{self, ActiveModel, Column, Entity as User};

/// Insert the user for this OIDC subject, or refresh their email/name if they
/// already exist. Returns the row either way.
pub async fn upsert(
    db: &DatabaseConnection,
    oidc_sub: &str,
    email: &str,
    name: &str,
) -> Result<users::Model, DbErr> {
    let model = ActiveModel {
        id: Set(Uuid::new_v4()),
        oidc_sub: Set(oidc_sub.to_string()),
        email: Set(email.to_string()),
        name: Set(name.to_string()),
        created_at: Set(chrono::Utc::now().fixed_offset()),
    };
    User::insert(model)
        .on_conflict(
            OnConflict::column(Column::OidcSub)
                .update_columns([Column::Email, Column::Name])
                .to_owned(),
        )
        .exec_with_returning(db)
        .await
}

/// Hot-path lookup used by the request guards: a plain read for known users,
/// falling back to an insert only the first time a subject is seen. Profile
/// refreshes happen at login via [`upsert`].
pub async fn resolve(
    db: &DatabaseConnection,
    oidc_sub: &str,
    email: &str,
    name: &str,
) -> Result<users::Model, DbErr> {
    match find_by_sub(db, oidc_sub).await? {
        Some(u) => Ok(u),
        None => upsert(db, oidc_sub, email, name).await,
    }
}

pub async fn find_by_sub(
    db: &DatabaseConnection,
    oidc_sub: &str,
) -> Result<Option<users::Model>, DbErr> {
    User::find()
        .filter(Column::OidcSub.eq(oidc_sub))
        .one(db)
        .await
}
