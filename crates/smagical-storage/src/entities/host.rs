//! 主机资产实体 (hosts)

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "hosts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub address: String,
    pub port: i32,
    pub parent_group_id: Option<String>,
    pub credential_id: Option<String>,
    pub status: String,
    pub ping_ms: i32,
    pub sort_order: i32,
    pub notes: String,

    // 认证参数 (敏感数据均由 DEK 加密为 enc:v1:... 格式)
    pub auth_type: String,
    pub username: Option<String>,
    pub password_enc: Option<String>,
    pub key_data_enc: Option<String>,
    pub key_passphrase_enc: Option<String>,

    // 网络代理与高级选项
    pub proxy_type: Option<String>,
    pub proxy_host: Option<String>,
    pub proxy_port: Option<i32>,
    pub proxy_username: Option<String>,
    pub proxy_password_enc: Option<String>,
    pub jump_chain_json: String,
    pub keepalive_interval: i32,
    pub connect_timeout: i32,
    pub initial_dir: Option<String>,
    pub startup_cmd: Option<String>,
    pub term_type: Option<String>,

    pub tags_json: String,
    pub version: i32,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
