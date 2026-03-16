use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(BrewEvents::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(BrewEvents::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(ColumnDef::new(BrewEvents::BrewId).uuid().not_null())
                    .col(ColumnDef::new(BrewEvents::EventType).string().not_null())
                    .col(ColumnDef::new(BrewEvents::Label).string().not_null())
                    .col(ColumnDef::new(BrewEvents::Notes).text())
                    .col(ColumnDef::new(BrewEvents::GravityAtEvent).double())
                    .col(ColumnDef::new(BrewEvents::TempAtEvent).double())
                    .col(
                        ColumnDef::new(BrewEvents::EventTime)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(BrewEvents::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .extra("DEFAULT now()"),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_brew_events_brew_id")
                            .from(BrewEvents::Table, BrewEvents::BrewId)
                            .to(Brews::Table, Brews::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_brew_events_brew_id_event_time")
                    .table(BrewEvents::Table)
                    .col(BrewEvents::BrewId)
                    .col(BrewEvents::EventTime)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(BrewEvents::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum BrewEvents {
    Table,
    Id,
    BrewId,
    EventType,
    Label,
    Notes,
    GravityAtEvent,
    TempAtEvent,
    EventTime,
    CreatedAt,
}

#[derive(DeriveIden)]
enum Brews {
    Table,
    Id,
}
