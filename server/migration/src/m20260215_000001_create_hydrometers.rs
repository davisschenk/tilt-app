use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Hydrometers::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Hydrometers::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(Hydrometers::Color)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Hydrometers::Name).string().null())
                    .col(
                        ColumnDef::new(Hydrometers::TempOffsetF)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(Hydrometers::GravityOffset)
                            .double()
                            .not_null()
                            .default(0.0),
                    )
                    .col(
                        ColumnDef::new(Hydrometers::CreatedAt)
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
            .drop_table(Table::drop().table(Hydrometers::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Hydrometers {
    Table,
    Id,
    Color,
    Name,
    TempOffsetF,
    GravityOffset,
    CreatedAt,
}
