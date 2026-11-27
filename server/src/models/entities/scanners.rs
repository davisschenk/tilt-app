use sea_orm::entity::prelude::*;
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, serde::Serialize)]
#[sea_orm(table_name = "scanners")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique_key = "scanners_owner_id_device_id_key")]
    pub owner_id: Uuid,
    #[sea_orm(unique_key = "scanners_owner_id_device_id_key")]
    pub device_id: String,
    pub payload: Json,
    pub last_seen_at: DateTimeWithTimeZone,
}
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
impl ActiveModelBehavior for ActiveModel {}
