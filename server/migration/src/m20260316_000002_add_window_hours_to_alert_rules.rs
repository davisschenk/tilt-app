use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(AlertRules::Table)
                    .add_column(
                        ColumnDef::new(AlertRules::WindowHours)
                            .integer()
                            .not_null()
                            .default(24),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(AlertRules::Table)
                    .drop_column(AlertRules::WindowHours)
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum AlertRules {
    Table,
    WindowHours,
}
