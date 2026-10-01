//! 代码片段实体 (snippets)

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "snippets")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub workspace_id: String,
    pub parent_group_id: Option<String>,
    pub title: String,
    pub content: String,
    pub language: String,
    pub tags_json: String,
    pub auto_execute: bool,
    pub description: String,
    pub is_favorite: bool,
    pub sort_order: i32,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
