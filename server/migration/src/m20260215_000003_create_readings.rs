use sea_orm_migration::prelude::*;

use super::m20260215_000001_create_hydrometers::Hydrometers;
use super::m20260215_000002_create_brews::Brews;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Readings::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Readings::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(ColumnDef::new(Readings::BrewId).uuid().null())
                    .col(ColumnDef::new(Readings::HydrometerId).uuid().not_null())
                    .col(ColumnDef::new(Readings::TemperatureF).double().not_null())
                    .col(ColumnDef::new(Readings::Gravity).double().not_null())
                    .col(ColumnDef::new(Readings::Rssi).small_integer().null())
                    .col(
                        ColumnDef::new(Readings::RecordedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Readings::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_readings_brew_id")
                            .from(Readings::Table, Readings::BrewId)
                            .to(Brews::Table, Brews::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_readings_hydrometer_id")
                            .from(Readings::Table, Readings::HydrometerId)
                            .to(Hydrometers::Table, Hydrometers::Id)
                            .on_delete(ForeignKeyAction::Restrict),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_readings_brew_id")
                    .table(Readings::Table)
                    .col(Readings::BrewId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_readings_hydrometer_id")
                    .table(Readings::Table)
                    .col(Readings::HydrometerId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_readings_recorded_at")
                    .table(Readings::Table)
                    .col(Readings::RecordedAt)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Readings::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Readings {
    Table,
    Id,
    BrewId,
    HydrometerId,
    TemperatureF,
    Gravity,
    Rssi,
    RecordedAt,
    CreatedAt,
}
