use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(EventAttachments::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(EventAttachments::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(EventAttachments::EventId).uuid().not_null())
                    .col(
                        ColumnDef::new(EventAttachments::Filename)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(EventAttachments::StoragePath)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(EventAttachments::ContentType)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(EventAttachments::SizeBytes)
                            .big_integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(EventAttachments::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_event_attachments_event_id")
                            .from(EventAttachments::Table, EventAttachments::EventId)
                            .to(BrewEvents::Table, BrewEvents::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(EventAttachments::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum EventAttachments {
    Table,
    Id,
    EventId,
    Filename,
    StoragePath,
    ContentType,
    SizeBytes,
    CreatedAt,
}

#[derive(DeriveIden)]
enum BrewEvents {
    Table,
    Id,
}
