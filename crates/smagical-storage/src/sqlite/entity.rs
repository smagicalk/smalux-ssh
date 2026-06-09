//! SQLite schema 的 SeaORM entity 定义。
//!
//! 这个文件只声明表和列，让 mapper/migration 复用同一批类型。不要在 entity 里写业务规则；
//! 业务规则应留在 `StorageManager` 或 mapper 中。

use sea_orm::entity::prelude::*;

pub mod schema_meta {
    //! schema 元数据表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "schema_meta")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub key: String,
        pub value: String,
        pub updated_at_unix_secs: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod host_group {
    //! 主机分组表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "host_groups")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub name: String,
        pub parent_id: Option<String>,
        pub sort_order: i32,
        pub created_at_unix_secs: i64,
        pub updated_at_unix_secs: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod host {
    //! 主机主表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "hosts")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub name: String,
        pub group_id: Option<String>,
        pub icon_key: String,
        pub address: String,
        pub port: i32,
        pub theme_override_toml: Option<String>,
        pub background_override_toml: Option<String>,
        pub sort_order: i32,
        pub created_at_unix_secs: i64,
        pub updated_at_unix_secs: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod host_tag {
    //! 主机标签表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "host_tags")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub host_id: String,
        pub tag: String,
        pub sort_order: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod host_auth {
    //! 主机认证引用表，只保存 SecretRef，不保存明文。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "host_auth")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub host_id: String,
        pub auth_kind: String,
        pub username: String,
        pub password_secret_ref: Option<String>,
        pub key_secret_ref: Option<String>,
        pub passphrase_secret_ref: Option<String>,
        pub certificate_secret_ref: Option<String>,
        pub agent_source: Option<String>,
        pub agent_pipe: Option<String>,
        pub key_hint: Option<String>,
        pub updated_at_unix_secs: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod host_proxy {
    //! 主机代理配置表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "host_proxy")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub host_id: String,
        pub proxy_kind: String,
        pub proxy_host: String,
        pub proxy_port: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod host_jump {
    //! 主机跳板机链路表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "host_jumps")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub host_id: String,
        pub jump_host_id: String,
        pub sort_order: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod credential {
    //! 凭据元数据表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "credentials")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub name: String,
        pub id: Option<String>,
        pub kind: String,
        pub group_id: Option<String>,
        pub username: Option<String>,
        pub secret_ref: Option<String>,
        pub key_algorithm: Option<String>,
        pub key_algorithm_raw: Option<String>,
        pub fingerprint: Option<String>,
        pub created_at_unix_secs: i64,
        pub updated_at_unix_secs: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod credential_inspection {
    //! 凭据内容解析缓存表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "credential_inspections")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub credential_id: String,
        pub kind: String,
        pub payload_hash: String,
        pub parser_version: i32,
        pub parse_error: Option<String>,
        pub key_algorithm: Option<String>,
        pub key_algorithm_raw: Option<String>,
        pub fingerprint: Option<String>,
        pub public_key: Option<String>,
        pub comment: Option<String>,
        pub encrypted: Option<bool>,
        pub password_length: Option<i32>,
        pub cert_type: Option<String>,
        pub serial: Option<String>,
        pub key_id: Option<String>,
        pub principals_text: Option<String>,
        pub valid_after_unix_secs: Option<i64>,
        pub valid_before_unix_secs: Option<i64>,
        pub ca_fingerprint: Option<String>,
        pub subject_fingerprint: Option<String>,
        pub critical_options_json: Option<String>,
        pub extensions_json: Option<String>,
        pub created_at_unix_secs: i64,
        pub updated_at_unix_secs: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod credential_group {
    //! 密钥分组表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "credential_groups")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub name: String,
        pub kind: String,
        pub parent_id: Option<String>,
        pub sort_order: i32,
        pub created_at_unix_secs: i64,
        pub updated_at_unix_secs: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod secret {
    //! 预留秘密数据表，后续加密存储会使用。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "secrets")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub secret_ref: String,
        pub secret_kind: String,
        pub encryption_version: i32,
        pub kdf: Option<String>,
        pub kdf_params_toml: Option<String>,
        pub salt: Option<Vec<u8>>,
        pub nonce: Option<Vec<u8>>,
        pub encrypted_payload: Option<Vec<u8>>,
        pub external_store: Option<String>,
        pub external_key: Option<String>,
        pub created_at_unix_secs: i64,
        pub updated_at_unix_secs: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod known_host {
    //! Known Hosts 安全记录表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "known_hosts")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub host: String,
        pub port: i32,
        pub key_algorithm: String,
        pub key_algorithm_raw: Option<String>,
        pub fingerprint: String,
        pub trusted: bool,
        pub created_at_unix_secs: i64,
        pub updated_at_unix_secs: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod command_history {
    //! 命令历史表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "command_history")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub host_id: Option<String>,
        pub command: String,
        pub working_directory: Option<String>,
        pub exit_code: Option<i32>,
        pub started_at_unix_secs: i64,
        pub duration_ms: Option<i64>,
        pub created_at_unix_secs: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod recent_connection {
    //! 最近连接表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "recent_connections")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub host_id: String,
        pub label: String,
        pub connected_at_unix_secs: i64,
        pub updated_at_unix_secs: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod snippet {
    //! 快捷命令主表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "snippets")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub name: String,
        pub description: Option<String>,
        /// 旧 schema 字段；新脚本内容保存在 snippet_implementations。
        pub command_template: String,
        pub scope_kind: String,
        pub scope_target_id: Option<String>,
        pub group_id: Option<String>,
        pub sort_order: i32,
        pub created_at_unix_secs: i64,
        pub updated_at_unix_secs: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod snippet_implementation {
    //! 快捷命令脚本实现表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "snippet_implementations")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub snippet_id: String,
        pub name: String,
        pub shell: String,
        pub shell_custom: Option<String>,
        pub command_template: String,
        pub notes: Option<String>,
        pub sort_order: i32,
        pub created_at_unix_secs: i64,
        pub updated_at_unix_secs: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod snippet_support_target {
    //! 快捷命令支持目标表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "snippet_support_targets")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub snippet_id: String,
        pub target_key: String,
        pub display_name: String,
        pub implementation_id: String,
        pub sort_order: i32,
        pub created_at_unix_secs: i64,
        pub updated_at_unix_secs: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod snippet_group {
    //! 快捷命令分组表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "snippet_groups")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub name: String,
        pub parent_id: Option<String>,
        pub sort_order: i32,
        pub created_at_unix_secs: i64,
        pub updated_at_unix_secs: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod snippet_variable {
    //! 快捷命令变量表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "snippet_variables")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub snippet_id: String,
        pub name: String,
        pub default_value: Option<String>,
        pub required: bool,
        pub sort_order: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod snippet_argument {
    //! 快捷命令上次参数表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "snippet_arguments")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        /// 旧 schema 遗留列；新逻辑按 implementation_id 读取参数。
        pub snippet_id: Option<String>,
        pub implementation_id: String,
        pub name: String,
        pub value: String,
        pub sort_order: i32,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod setting {
    //! 应用设置表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "settings")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub key: String,
        pub value_toml: String,
        pub updated_at_unix_secs: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod theme_profile {
    //! 主题资料表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "theme_profiles")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub name: String,
        pub profile_toml: String,
        pub builtin: bool,
        pub created_at_unix_secs: i64,
        pub updated_at_unix_secs: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod workspace_state {
    //! 工作区恢复快照表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "workspace_state")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub workspace_key: String,
        pub state_toml: String,
        pub updated_at_unix_secs: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod sftp_bookmark {
    //! SFTP 书签表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "sftp_bookmarks")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: String,
        pub host_id: String,
        pub label: String,
        pub remote_path: String,
        pub sort_order: i32,
        pub created_at_unix_secs: i64,
        pub updated_at_unix_secs: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod tunnel_rule {
    //! SSH 隧道规则表。

    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "tunnel_rules")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub name: String,
        pub kind: String,
        pub bind_host: String,
        pub bind_port: i32,
        pub target_host: String,
        pub target_port: i32,
        pub auto_start: bool,
        pub sort_order: i32,
        pub created_at_unix_secs: i64,
        pub updated_at_unix_secs: i64,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
