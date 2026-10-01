//! 历史会话屏幕快照大文本实体 (history_snapshots)

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "history_snapshots")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub history_id: String,
    pub content: String,
    pub max_lines: i32,
    pub saved_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
