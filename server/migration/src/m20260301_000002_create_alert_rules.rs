use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(AlertRules::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AlertRules::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(ColumnDef::new(AlertRules::Name).string().not_null())
                    .col(ColumnDef::new(AlertRules::BrewId).uuid())
                    .col(ColumnDef::new(AlertRules::HydrometerId).uuid())
                    .col(ColumnDef::new(AlertRules::Metric).string().not_null())
                    .col(ColumnDef::new(AlertRules::Operator).string().not_null())
                    .col(ColumnDef::new(AlertRules::Threshold).double().not_null())
                    .col(ColumnDef::new(AlertRules::AlertTargetId).uuid().not_null())
                    .col(
                        ColumnDef::new(AlertRules::Enabled)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(AlertRules::CooldownMinutes)
                            .integer()
                            .not_null()
                            .default(60),
                    )
                    .col(ColumnDef::new(AlertRules::LastTriggeredAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(AlertRules::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .col(
                        ColumnDef::new(AlertRules::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_alert_rules_brew_id")
                            .from(AlertRules::Table, AlertRules::BrewId)
                            .to(Brews::Table, Brews::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_alert_rules_hydrometer_id")
                            .from(AlertRules::Table, AlertRules::HydrometerId)
                            .to(Hydrometers::Table, Hydrometers::Id)
                            .on_delete(ForeignKeyAction::SetNull),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_alert_rules_alert_target_id")
                            .from(AlertRules::Table, AlertRules::AlertTargetId)
                            .to(AlertTargets::Table, AlertTargets::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_alert_rules_brew_id")
                    .table(AlertRules::Table)
                    .col(AlertRules::BrewId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_alert_rules_hydrometer_id")
                    .table(AlertRules::Table)
                    .col(AlertRules::HydrometerId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_alert_rules_alert_target_id")
                    .table(AlertRules::Table)
                    .col(AlertRules::AlertTargetId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(AlertRules::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum AlertRules {
    Table,
    Id,
    Name,
    BrewId,
    HydrometerId,
    Metric,
    Operator,
    Threshold,
    AlertTargetId,
    Enabled,
    CooldownMinutes,
    LastTriggeredAt,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum Brews {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum Hydrometers {
    Table,
    Id,
}

#[derive(DeriveIden)]
enum AlertTargets {
    Table,
    Id,
}
