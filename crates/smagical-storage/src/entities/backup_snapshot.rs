//! 容灾备份快照实体 (backup_snapshots)

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "backup_snapshots")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub task_id: String,
    pub timestamp: String,
    pub epoch_secs: i64,
    pub size_str: String,
    pub size_bytes: i64,
    pub remark: String,
    pub hash: String,
    pub remote_id: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
