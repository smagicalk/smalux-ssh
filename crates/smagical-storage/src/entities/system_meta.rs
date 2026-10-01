//! 系统元数据与数据库版本迁移实体 (system_meta)

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "system_meta")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub meta_key: String,
    pub meta_value: String,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
