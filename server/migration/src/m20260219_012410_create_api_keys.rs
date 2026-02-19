use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ApiKeys::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ApiKeys::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(ColumnDef::new(ApiKeys::Name).string().not_null())
                    .col(ColumnDef::new(ApiKeys::KeyHash).string().not_null())
                    .col(ColumnDef::new(ApiKeys::Prefix).string_len(8).not_null())
                    .col(ColumnDef::new(ApiKeys::CreatedBy).string().not_null())
                    .col(ColumnDef::new(ApiKeys::LastUsedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(ApiKeys::ExpiresAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(ApiKeys::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_api_keys_key_hash")
                    .table(ApiKeys::Table)
                    .col(ApiKeys::KeyHash)
                    .unique()
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_api_keys_created_by")
                    .table(ApiKeys::Table)
                    .col(ApiKeys::CreatedBy)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_api_keys_expires_at")
                    .table(ApiKeys::Table)
                    .col(ApiKeys::ExpiresAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ApiKeys::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ApiKeys {
    Table,
    Id,
    Name,
    KeyHash,
    Prefix,
    CreatedBy,
    LastUsedAt,
    ExpiresAt,
    CreatedAt,
}
