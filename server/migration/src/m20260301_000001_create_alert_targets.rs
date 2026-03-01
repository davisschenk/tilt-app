use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AlertTargets::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AlertTargets::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(ColumnDef::new(AlertTargets::Name).string().not_null())
                    .col(ColumnDef::new(AlertTargets::Url).string().not_null())
                    .col(ColumnDef::new(AlertTargets::Format).string().not_null())
                    .col(ColumnDef::new(AlertTargets::SecretHeader).string())
                    .col(
                        ColumnDef::new(AlertTargets::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(AlertTargets::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(AlertTargets::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AlertTargets::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum AlertTargets {
    Table,
    Id,
    Name,
    Url,
    Format,
    SecretHeader,
    Enabled,
    CreatedAt,
    UpdatedAt,
}
