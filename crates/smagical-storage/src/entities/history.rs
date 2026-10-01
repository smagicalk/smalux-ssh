//! 历史会话记录实体 (history)

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "history")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub workspace_id: String,
    pub host_id: Option<String>,
    pub title: String,
    pub address: String,
    pub port: i32,
    pub username: String,
    pub session_type: String,
    pub connected_at: i64,
    pub disconnected_at: Option<i64>,
    pub duration_secs: i64,
    pub exit_status: String,
    pub error_msg: Option<String>,
    pub is_pinned: bool,
    pub connect_count: i32,
    pub has_snapshot: bool,
    pub snapshot_lines: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
