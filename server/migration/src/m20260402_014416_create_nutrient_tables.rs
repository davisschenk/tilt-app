use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(NutrientSchedules::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(NutrientSchedules::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(NutrientSchedules::BrewId)
                            .uuid()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(NutrientSchedules::BatchSizeGallons)
                            .double()
                            .not_null(),
                    )
                    .col(ColumnDef::new(NutrientSchedules::Og).double().not_null())
                    .col(
                        ColumnDef::new(NutrientSchedules::NitrogenRequirement)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NutrientSchedules::NutrientProtocol)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NutrientSchedules::TotalYanPpm)
                            .double()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NutrientSchedules::GoFermYanOffsetPpm)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(NutrientSchedules::FruitYanOffsetPpm)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(NutrientSchedules::EffectiveYanPpm)
                            .double()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NutrientSchedules::GoFermGrams)
                            .double()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NutrientSchedules::YeastGrams)
                            .double()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NutrientSchedules::RehydrationWaterMl)
                            .double()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NutrientSchedules::OneThirdBreakSg)
                            .double()
                            .not_null(),
                    )
                    .col(ColumnDef::new(NutrientSchedules::AlertTargetId).uuid())
                    .col(
                        ColumnDef::new(NutrientSchedules::MaxDosageCapped)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(NutrientSchedules::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(NutrientSchedules::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_nutrient_schedules_brew_id")
                            .from(NutrientSchedules::Table, NutrientSchedules::BrewId)
                            .to(Brews::Table, Brews::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_nutrient_schedules_alert_target_id")
                            .from(NutrientSchedules::Table, NutrientSchedules::AlertTargetId)
                            .to(AlertTargets::Table, AlertTargets::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(NutrientAdditions::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(NutrientAdditions::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(NutrientAdditions::ScheduleId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NutrientAdditions::AdditionNumber)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(NutrientAdditions::FermaidOGrams)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(NutrientAdditions::FermaidKGrams)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(NutrientAdditions::DapGrams)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(NutrientAdditions::TriggerType)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(NutrientAdditions::TargetHours).double())
                    .col(ColumnDef::new(NutrientAdditions::TargetGravity).double())
                    .col(ColumnDef::new(NutrientAdditions::NotifiedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(NutrientAdditions::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_nutrient_additions_schedule_id")
                            .from(NutrientAdditions::Table, NutrientAdditions::ScheduleId)
                            .to(NutrientSchedules::Table, NutrientSchedules::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_nutrient_additions_schedule_id")
                    .table(NutrientAdditions::Table)
                    .col(NutrientAdditions::ScheduleId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(NutrientAdditions::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(NutrientSchedules::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum NutrientSchedules {
    Table,
    Id,
    BrewId,
    BatchSizeGallons,
    Og,
    NitrogenRequirement,
    NutrientProtocol,
    TotalYanPpm,
    GoFermYanOffsetPpm,
    FruitYanOffsetPpm,
    EffectiveYanPpm,
    GoFermGrams,
    YeastGrams,
    RehydrationWaterMl,
    OneThirdBreakSg,
    AlertTargetId,
    MaxDosageCapped,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum NutrientAdditions {
    Table,
    Id,
    ScheduleId,
    AdditionNumber,
    FermaidOGrams,
    FermaidKGrams,
    DapGrams,
    TriggerType,
    TargetHours,
    TargetGravity,
    NotifiedAt,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Brews {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum AlertTargets {
    Table,
    Id,
}
