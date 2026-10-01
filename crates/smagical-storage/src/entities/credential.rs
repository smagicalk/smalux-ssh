//! 凭据保险库实体 (credentials)

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "credentials")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub workspace_id: String,
    pub name: String,
    pub cred_type: String,
    pub algorithm: String,
    pub username: Option<String>,
    pub secret_data_enc: String,
    pub passphrase_enc: Option<String>,
    pub public_key: Option<String>,
    pub fingerprint: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
