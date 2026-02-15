use sea_orm_migration::prelude::*;

use super::m20260215_000001_create_hydrometers::Hydrometers;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Brews::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Brews::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(ColumnDef::new(Brews::Name).string().not_null())
                    .col(ColumnDef::new(Brews::Style).string().null())
                    .col(ColumnDef::new(Brews::Og).double().null())
                    .col(ColumnDef::new(Brews::Fg).double().null())
                    .col(ColumnDef::new(Brews::TargetFg).double().null())
                    .col(ColumnDef::new(Brews::Abv).double().null())
                    .col(
                        ColumnDef::new(Brews::Status)
                            .string()
                            .not_null()
                            .default("Active"),
                    )
                    .col(
                        ColumnDef::new(Brews::StartDate)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(Brews::EndDate)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .col(ColumnDef::new(Brews::Notes).text().null())
                    .col(ColumnDef::new(Brews::HydrometerId).uuid().not_null())
                    .col(
                        ColumnDef::new(Brews::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(Brews::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_brews_hydrometer_id")
                            .from(Brews::Table, Brews::HydrometerId)
                            .to(Hydrometers::Table, Hydrometers::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_brews_hydrometer_id")
                    .table(Brews::Table)
                    .col(Brews::HydrometerId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_brews_status")
                    .table(Brews::Table)
                    .col(Brews::Status)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Brews::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum Brews {
    Table,
    Id,
    Name,
    Style,
    Og,
    Fg,
    TargetFg,
    Abv,
    Status,
    StartDate,
    EndDate,
    Notes,
    HydrometerId,
    CreatedAt,
    UpdatedAt,
}
