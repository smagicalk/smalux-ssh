//! 容灾备份任务实体 (backup_tasks)

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "backup_tasks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub backup_type: String,
    pub endpoint: String,
    pub auth_user: String,
    /// 经由安全保险库加密保护的凭据口令/Token (密文存储)
    pub auth_secret_enc: String,
    pub strategy: String,
    pub retention: String,
    pub enabled: bool,
    pub last_backup_time: String,
    pub snapshot_count: i32,
    pub last_status: String,
    pub last_error: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
