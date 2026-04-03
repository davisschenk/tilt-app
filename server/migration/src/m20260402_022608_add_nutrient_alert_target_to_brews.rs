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
                    .add_column(ColumnDef::new(Brews::NutrientAlertTargetId).uuid().null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Brews::Table)
                    .drop_column(Brews::NutrientAlertTargetId)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Brews {
    Table,
    NutrientAlertTargetId,
}
