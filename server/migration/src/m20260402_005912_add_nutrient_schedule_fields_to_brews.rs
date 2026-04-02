use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Brews::Table)
                    .add_column(ColumnDef::new(Brews::BatchSizeGallons).double().null())
                    .add_column(
                        ColumnDef::new(Brews::YeastNitrogenRequirement)
                            .string()
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(Brews::PitchTime)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .add_column(ColumnDef::new(Brews::NutrientProtocol).string().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Brews::Table)
                    .drop_column(Brews::BatchSizeGallons)
                    .drop_column(Brews::YeastNitrogenRequirement)
                    .drop_column(Brews::PitchTime)
                    .drop_column(Brews::NutrientProtocol)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Brews {
    Table,
    BatchSizeGallons,
    YeastNitrogenRequirement,
    PitchTime,
    NutrientProtocol,
}
