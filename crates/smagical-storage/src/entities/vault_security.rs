//! 安全保险库主控实体 (vault_security)

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "vault_security")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub kdf_algorithm: String,
    pub kdf_salt: String,
    pub kdf_iterations: i32,
    pub kdf_memory_kib: i32,
    pub kdf_parallelism: i32,
    pub canary_ciphertext: String,
    pub encrypted_dek: String,
    pub dek_version: i32,
    pub password_hint: String,
    pub last_rotated_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
