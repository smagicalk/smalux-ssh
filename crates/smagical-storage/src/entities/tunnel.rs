//! 网络隧道与代理实体 (tunnels)

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "tunnels")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub tunnel_type: String,
    pub local_bind: String,
    pub local_port: i32,
    pub remote_host: String,
    pub remote_port: i32,
    pub associated_host_id: Option<String>,
    pub run_mode: String,
    pub is_running: bool,
    pub jump_hops_json: String,
    pub record_json: String,
    pub notes: String,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
