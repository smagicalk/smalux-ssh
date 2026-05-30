//! `StorageManager` 与 SQLite 业务表之间的映射。
//!
//! `StorageManager` 是应用内部的内存快照，SQLite 是落盘结构。这个模块是两者之间唯一的
//! 编解码层：加载时把多表数据聚合成内存结构，保存时把内存结构拆回多张业务表。

use std::collections::HashMap;

use sea_orm::{
    ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
};
use uuid::Uuid;

use smagical_config::AppConfig;
use smagical_core::{
    AgentSource, AuthProfile, CommandHistoryId, CommandHistoryItem, CredentialKind,
    CredentialMetadata, GroupId, Host, HostGroup, HostId, JumpProfile, KeyAlgorithm,
    KnownHostEntry, ProxyProfile, RecentConnection, SecretRef, SftpBookmark, Snippet,
    SnippetArgument, SnippetId, SnippetScope, SnippetVariable, TunnelKind, TunnelRule,
    WorkspaceState,
};

use super::{
    APP_CONFIG_SETTING_KEY, DEFAULT_WORKSPACE_KEY, clear_entities, current_unix_secs, entity,
};
use crate::{StorageManager, StoragePersistenceError, ThemeProfileRecord};

pub(super) async fn load_storage(
    db: &DatabaseConnection,
) -> Result<StorageManager, StoragePersistenceError> {
    // 加载顺序先读独立数据，再通过 StorageManager 方法导入有不变量的集合。
    let mut storage = StorageManager::default();
    storage.app_config = load_app_config(db).await?;
    storage.groups = load_groups(db).await?;
    storage.hosts = load_hosts(db).await?;
    storage.credentials = load_credentials(db).await?;
    storage.known_hosts = load_known_hosts(db).await?;

    for connection in load_recent_connections(db).await? {
        storage.record_recent_connection(connection);
    }
    for item in load_command_history(db).await? {
        storage.add_command_history(item);
    }

    storage.snippets = load_snippets(db).await?;
    storage.sftp_bookmarks = load_sftp_bookmarks(db).await?;
    for rule in load_tunnel_rules(db).await? {
        storage.upsert_tunnel_rule(rule);
    }
    storage.themes = load_themes(db).await?;
    storage.workspace = load_workspace(db).await?;

    Ok(storage)
}

pub(super) async fn save_storage(
    db: &DatabaseConnection,
    storage: &StorageManager,
) -> Result<(), StoragePersistenceError> {
    // 当前保存策略是整体替换，简单可靠；未来要做增量保存时应保持这里的外部语义不变。
    clear_business_tables(db).await?;
    save_settings(db, storage).await?;
    save_groups(db, &storage.groups).await?;
    save_hosts(db, &storage.hosts).await?;
    save_credentials(db, &storage.credentials).await?;
    save_known_hosts(db, &storage.known_hosts).await?;
    save_history(db, storage).await?;
    save_snippets(db, &storage.snippets).await?;
    save_sftp_bookmarks(db, &storage.sftp_bookmarks).await?;
    save_tunnel_rules(db, &storage.tunnel_rules).await?;
    save_themes(db, storage).await?;
    save_workspace(db, storage.workspace.as_ref()).await?;
    Ok(())
}

async fn clear_business_tables(db: &DatabaseConnection) -> Result<(), StoragePersistenceError> {
    // 按依赖关系从子表到父表清空，避免外键约束失败。
    clear_entities::<entity::workspace_state::Entity>(db).await?;
    clear_entities::<entity::theme_profile::Entity>(db).await?;
    clear_entities::<entity::setting::Entity>(db).await?;
    clear_entities::<entity::tunnel_rule::Entity>(db).await?;
    clear_entities::<entity::sftp_bookmark::Entity>(db).await?;
    clear_entities::<entity::snippet_argument::Entity>(db).await?;
    clear_entities::<entity::snippet_variable::Entity>(db).await?;
    clear_entities::<entity::snippet::Entity>(db).await?;
    clear_entities::<entity::recent_connection::Entity>(db).await?;
    clear_entities::<entity::command_history::Entity>(db).await?;
    clear_entities::<entity::known_host::Entity>(db).await?;
    clear_entities::<entity::secret::Entity>(db).await?;
    clear_entities::<entity::credential::Entity>(db).await?;
    clear_entities::<entity::host_jump::Entity>(db).await?;
    clear_entities::<entity::host_proxy::Entity>(db).await?;
    clear_entities::<entity::host_auth::Entity>(db).await?;
    clear_entities::<entity::host_tag::Entity>(db).await?;
    clear_entities::<entity::host::Entity>(db).await?;
    clear_entities::<entity::host_group::Entity>(db).await?;
    Ok(())
}

async fn load_app_config(db: &DatabaseConnection) -> Result<AppConfig, StoragePersistenceError> {
    // 配置以 TOML 存在 settings 表，便于新增配置字段时向后兼容。
    let Some(model) = entity::setting::Entity::find()
        .filter(entity::setting::Column::Key.eq(APP_CONFIG_SETTING_KEY))
        .one(db)
        .await?
    else {
        return Ok(AppConfig::default());
    };

    Ok(toml::from_str(&model.value_toml)?)
}

async fn save_settings(
    db: &DatabaseConnection,
    storage: &StorageManager,
) -> Result<(), StoragePersistenceError> {
    let now = current_unix_secs();
    // app_config 作为整体 TOML 保存，避免为每个设置项建立独立列。
    entity::setting::Entity::insert(entity::setting::ActiveModel {
        key: Set(APP_CONFIG_SETTING_KEY.to_owned()),
        value_toml: Set(toml::to_string(&storage.app_config)?),
        updated_at_unix_secs: Set(now),
    })
    .exec(db)
    .await?;

    Ok(())
}

async fn load_themes(
    db: &DatabaseConnection,
) -> Result<Vec<ThemeProfileRecord>, StoragePersistenceError> {
    // 按名称排序，保证设置页和导出结果稳定。
    entity::theme_profile::Entity::find()
        .order_by_asc(entity::theme_profile::Column::Name)
        .all(db)
        .await?
        .into_iter()
        .map(|model| {
            Ok(ThemeProfileRecord {
                name: model.name,
                profile_toml: model.profile_toml,
                builtin: model.builtin,
            })
        })
        .collect()
}

async fn save_themes(
    db: &DatabaseConnection,
    storage: &StorageManager,
) -> Result<(), StoragePersistenceError> {
    let now = current_unix_secs();
    for theme in &storage.themes {
        entity::theme_profile::Entity::insert(entity::theme_profile::ActiveModel {
            name: Set(theme.name.clone()),
            profile_toml: Set(theme.profile_toml.clone()),
            builtin: Set(theme.builtin),
            created_at_unix_secs: Set(now),
            updated_at_unix_secs: Set(now),
        })
        .exec(db)
        .await?;
    }

    Ok(())
}

async fn load_groups(db: &DatabaseConnection) -> Result<Vec<HostGroup>, StoragePersistenceError> {
    entity::host_group::Entity::find()
        .order_by_asc(entity::host_group::Column::SortOrder)
        .all(db)
        .await?
        .into_iter()
        .map(|model| {
            Ok(HostGroup {
                id: GroupId(parse_uuid(&model.id)?),
                name: model.name,
                parent_id: model
                    .parent_id
                    .as_deref()
                    .map(parse_uuid)
                    .transpose()?
                    .map(GroupId),
            })
        })
        .collect()
}

async fn save_groups(
    db: &DatabaseConnection,
    groups: &[HostGroup],
) -> Result<(), StoragePersistenceError> {
    let now = current_unix_secs();
    for (index, group) in groups.iter().enumerate() {
        // sort_order 保存当前内存顺序，树形层级由 parent_id 表达。
        entity::host_group::Entity::insert(entity::host_group::ActiveModel {
            id: Set(group.id.0.to_string()),
            name: Set(group.name.clone()),
            parent_id: Set(group.parent_id.map(|id| id.0.to_string())),
            sort_order: Set(index as i32),
            created_at_unix_secs: Set(now),
            updated_at_unix_secs: Set(now),
        })
        .exec(db)
        .await?;
    }
    Ok(())
}

async fn load_hosts(db: &DatabaseConnection) -> Result<Vec<Host>, StoragePersistenceError> {
    // 主机本体、认证、代理、标签、跳板机分表存储，加载时先按 host_id 建索引。
    let auth_by_host: HashMap<_, _> = entity::host_auth::Entity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|model| (model.host_id.clone(), model))
        .collect();
    let proxy_by_host: HashMap<_, _> = entity::host_proxy::Entity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|model| (model.host_id.clone(), model))
        .collect();
    let mut tags_by_host: HashMap<String, Vec<entity::host_tag::Model>> = HashMap::new();
    for model in entity::host_tag::Entity::find()
        .order_by_asc(entity::host_tag::Column::SortOrder)
        .all(db)
        .await?
    {
        tags_by_host
            .entry(model.host_id.clone())
            .or_default()
            .push(model);
    }
    let mut jumps_by_host: HashMap<String, Vec<entity::host_jump::Model>> = HashMap::new();
    for model in entity::host_jump::Entity::find()
        .order_by_asc(entity::host_jump::Column::SortOrder)
        .all(db)
        .await?
    {
        jumps_by_host
            .entry(model.host_id.clone())
            .or_default()
            .push(model);
    }

    let mut hosts = Vec::new();
    for model in entity::host::Entity::find()
        .order_by_asc(entity::host::Column::SortOrder)
        .all(db)
        .await?
    {
        let auth = auth_by_host
            .get(&model.id)
            .map(auth_from_model)
            .transpose()?
            .unwrap_or_else(default_auth);
        let proxy = proxy_by_host
            .get(&model.id)
            .map(proxy_from_model)
            .transpose()?;
        let tags = tags_by_host
            .remove(&model.id)
            .unwrap_or_default()
            .into_iter()
            .map(|tag| tag.tag)
            .collect();
        let jumps = jumps_by_host
            .remove(&model.id)
            .unwrap_or_default()
            .into_iter()
            .map(|jump| {
                Ok(JumpProfile {
                    host_id: HostId(parse_uuid(&jump.jump_host_id)?),
                })
            })
            .collect::<Result<Vec<_>, StoragePersistenceError>>()?;

        hosts.push(Host {
            id: HostId(parse_uuid(&model.id)?),
            name: model.name,
            group_id: model
                .group_id
                .as_deref()
                .map(parse_uuid)
                .transpose()?
                .map(GroupId),
            icon_key: model.icon_key,
            tags,
            address: model.address,
            port: to_u16(model.port)?,
            auth,
            proxy,
            jumps,
            theme_override: decode_optional_toml(model.theme_override_toml.as_deref())?,
            background_override: decode_optional_toml(model.background_override_toml.as_deref())?,
        });
    }
    Ok(hosts)
}

async fn save_hosts(
    db: &DatabaseConnection,
    hosts: &[Host],
) -> Result<(), StoragePersistenceError> {
    let now = current_unix_secs();
    for (index, host) in hosts.iter().enumerate() {
        let host_id = host.id.0.to_string();
        // host 表保存主记录；认证、代理、标签和跳板机用子表表达一对多/可选关系。
        entity::host::Entity::insert(entity::host::ActiveModel {
            id: Set(host_id.clone()),
            name: Set(host.name.clone()),
            group_id: Set(host.group_id.map(|id| id.0.to_string())),
            icon_key: Set(host.icon_key.clone()),
            address: Set(host.address.clone()),
            port: Set(host.port as i32),
            theme_override_toml: Set(encode_optional_toml(host.theme_override.as_ref())?),
            background_override_toml: Set(encode_optional_toml(host.background_override.as_ref())?),
            sort_order: Set(index as i32),
            created_at_unix_secs: Set(now),
            updated_at_unix_secs: Set(now),
        })
        .exec(db)
        .await?;

        save_host_auth(db, &host_id, &host.auth, now).await?;
        save_host_proxy(db, &host_id, host.proxy.as_ref()).await?;

        for (tag_index, tag) in host.tags.iter().enumerate() {
            // tag id 使用 host_id + 顺序，整体替换保存时足够稳定。
            entity::host_tag::Entity::insert(entity::host_tag::ActiveModel {
                id: Set(format!("{host_id}:tag:{tag_index}")),
                host_id: Set(host_id.clone()),
                tag: Set(tag.clone()),
                sort_order: Set(tag_index as i32),
            })
            .exec(db)
            .await?;
        }

        for (jump_index, jump) in host.jumps.iter().enumerate() {
            // 跳板机只保存目标 host_id，不复制被引用主机的配置。
            entity::host_jump::Entity::insert(entity::host_jump::ActiveModel {
                id: Set(format!("{host_id}:jump:{jump_index}")),
                host_id: Set(host_id.clone()),
                jump_host_id: Set(jump.host_id.0.to_string()),
                sort_order: Set(jump_index as i32),
            })
            .exec(db)
            .await?;
        }
    }
    Ok(())
}

async fn save_host_auth(
    db: &DatabaseConnection,
    host_id: &str,
    auth: &AuthProfile,
    now: i64,
) -> Result<(), StoragePersistenceError> {
    // 认证表只保存 SecretRef 字符串，不保存明文密码、私钥或证书。
    let model = match auth {
        AuthProfile::Password { username, secret } => entity::host_auth::ActiveModel {
            host_id: Set(host_id.to_owned()),
            auth_kind: Set("password".to_owned()),
            username: Set(username.clone()),
            password_secret_ref: Set(Some(secret.0.clone())),
            key_secret_ref: Set(None),
            passphrase_secret_ref: Set(None),
            certificate_secret_ref: Set(None),
            agent_source: Set(None),
            agent_pipe: Set(None),
            key_hint: Set(None),
            updated_at_unix_secs: Set(now),
        },
        AuthProfile::Key {
            username,
            key,
            passphrase,
        } => entity::host_auth::ActiveModel {
            host_id: Set(host_id.to_owned()),
            auth_kind: Set("key".to_owned()),
            username: Set(username.clone()),
            password_secret_ref: Set(None),
            key_secret_ref: Set(Some(key.0.clone())),
            passphrase_secret_ref: Set(passphrase.as_ref().map(|reference| reference.0.clone())),
            certificate_secret_ref: Set(None),
            agent_source: Set(None),
            agent_pipe: Set(None),
            key_hint: Set(None),
            updated_at_unix_secs: Set(now),
        },
        AuthProfile::Agent {
            username,
            source,
            key_hint,
        } => {
            let (agent_source, agent_pipe) = agent_source_to_parts(source);
            entity::host_auth::ActiveModel {
                host_id: Set(host_id.to_owned()),
                auth_kind: Set("agent".to_owned()),
                username: Set(username.clone()),
                password_secret_ref: Set(None),
                key_secret_ref: Set(None),
                passphrase_secret_ref: Set(None),
                certificate_secret_ref: Set(None),
                agent_source: Set(Some(agent_source)),
                agent_pipe: Set(agent_pipe),
                key_hint: Set(key_hint.clone()),
                updated_at_unix_secs: Set(now),
            }
        }
        AuthProfile::Certificate {
            username,
            key,
            passphrase,
            certificate,
        } => entity::host_auth::ActiveModel {
            host_id: Set(host_id.to_owned()),
            auth_kind: Set("certificate".to_owned()),
            username: Set(username.clone()),
            password_secret_ref: Set(None),
            key_secret_ref: Set(Some(key.0.clone())),
            passphrase_secret_ref: Set(passphrase.as_ref().map(|reference| reference.0.clone())),
            certificate_secret_ref: Set(Some(certificate.0.clone())),
            agent_source: Set(None),
            agent_pipe: Set(None),
            key_hint: Set(None),
            updated_at_unix_secs: Set(now),
        },
    };

    entity::host_auth::Entity::insert(model).exec(db).await?;
    Ok(())
}

async fn save_host_proxy(
    db: &DatabaseConnection,
    host_id: &str,
    proxy: Option<&ProxyProfile>,
) -> Result<(), StoragePersistenceError> {
    let Some(proxy) = proxy else {
        // 没有代理时不写 host_proxy 行，加载时自然得到 None。
        return Ok(());
    };
    let (proxy_kind, proxy_host, proxy_port) = match proxy {
        ProxyProfile::Socks5 { host, port } => ("socks5", host, *port),
        ProxyProfile::Http { host, port } => ("http", host, *port),
    };
    entity::host_proxy::Entity::insert(entity::host_proxy::ActiveModel {
        host_id: Set(host_id.to_owned()),
        proxy_kind: Set(proxy_kind.to_owned()),
        proxy_host: Set(proxy_host.clone()),
        proxy_port: Set(proxy_port as i32),
    })
    .exec(db)
    .await?;
    Ok(())
}

async fn load_credentials(
    db: &DatabaseConnection,
) -> Result<Vec<CredentialMetadata>, StoragePersistenceError> {
    // 凭据表保存元数据和 SecretRef，真实 secret 数据预留在 secrets 表。
    entity::credential::Entity::find()
        .order_by_asc(entity::credential::Column::Name)
        .all(db)
        .await?
        .into_iter()
        .map(|model| {
            Ok(CredentialMetadata {
                name: model.name,
                kind: credential_kind_from_str(&model.kind),
                username: model.username,
                secret: model.secret_ref.map(SecretRef),
                key_algorithm: model
                    .key_algorithm
                    .as_deref()
                    .map(|kind| key_algorithm_from_parts(kind, model.key_algorithm_raw.as_deref())),
                fingerprint: model.fingerprint,
            })
        })
        .collect()
}

async fn save_credentials(
    db: &DatabaseConnection,
    credentials: &[CredentialMetadata],
) -> Result<(), StoragePersistenceError> {
    let now = current_unix_secs();
    for credential in credentials {
        // key_algorithm 分成标准 kind 和 raw，既能查询常见算法，也保留未知算法原文。
        let (key_algorithm, key_algorithm_raw) = credential
            .key_algorithm
            .as_ref()
            .map(key_algorithm_to_parts)
            .unwrap_or((None, None));
        entity::credential::Entity::insert(entity::credential::ActiveModel {
            name: Set(credential.name.clone()),
            kind: Set(credential_kind_to_str(&credential.kind).to_owned()),
            username: Set(credential.username.clone()),
            secret_ref: Set(credential
                .secret
                .as_ref()
                .map(|reference| reference.0.clone())),
            key_algorithm: Set(key_algorithm),
            key_algorithm_raw: Set(key_algorithm_raw),
            fingerprint: Set(credential.fingerprint.clone()),
            created_at_unix_secs: Set(now),
            updated_at_unix_secs: Set(now),
        })
        .exec(db)
        .await?;
    }
    Ok(())
}

async fn load_known_hosts(
    db: &DatabaseConnection,
) -> Result<Vec<KnownHostEntry>, StoragePersistenceError> {
    // Known Hosts 按 host 排序，保证安全面板展示稳定。
    entity::known_host::Entity::find()
        .order_by_asc(entity::known_host::Column::Host)
        .all(db)
        .await?
        .into_iter()
        .map(|model| {
            Ok(KnownHostEntry {
                host: model.host,
                port: to_u16(model.port)?,
                key_algorithm: key_algorithm_from_parts(
                    &model.key_algorithm,
                    model.key_algorithm_raw.as_deref(),
                ),
                fingerprint: model.fingerprint,
                trusted: model.trusted,
            })
        })
        .collect()
}

async fn save_known_hosts(
    db: &DatabaseConnection,
    entries: &[KnownHostEntry],
) -> Result<(), StoragePersistenceError> {
    let now = current_unix_secs();
    for entry in entries {
        // id 使用 host:port，当前模型同一 host/port 只保留一条记录。
        let (key_algorithm, key_algorithm_raw) = key_algorithm_to_parts(&entry.key_algorithm);
        entity::known_host::Entity::insert(entity::known_host::ActiveModel {
            id: Set(format!("{}:{}", entry.host, entry.port)),
            host: Set(entry.host.clone()),
            port: Set(entry.port as i32),
            key_algorithm: Set(key_algorithm.unwrap_or_else(|| "unknown".to_owned())),
            key_algorithm_raw: Set(key_algorithm_raw),
            fingerprint: Set(entry.fingerprint.clone()),
            trusted: Set(entry.trusted),
            created_at_unix_secs: Set(now),
            updated_at_unix_secs: Set(now),
        })
        .exec(db)
        .await?;
    }
    Ok(())
}

async fn load_recent_connections(
    db: &DatabaseConnection,
) -> Result<Vec<RecentConnection>, StoragePersistenceError> {
    entity::recent_connection::Entity::find()
        .order_by_asc(entity::recent_connection::Column::ConnectedAtUnixSecs)
        .all(db)
        .await?
        .into_iter()
        .map(|model| {
            Ok(RecentConnection {
                host_id: HostId(parse_uuid(&model.host_id)?),
                label: model.label,
                connected_at_unix_secs: to_u64(model.connected_at_unix_secs)?,
            })
        })
        .collect()
}

async fn load_command_history(
    db: &DatabaseConnection,
) -> Result<Vec<CommandHistoryItem>, StoragePersistenceError> {
    entity::command_history::Entity::find()
        .order_by_asc(entity::command_history::Column::StartedAtUnixSecs)
        .all(db)
        .await?
        .into_iter()
        .map(|model| {
            Ok(CommandHistoryItem {
                id: CommandHistoryId(parse_uuid(&model.id)?),
                host_id: model
                    .host_id
                    .as_deref()
                    .map(parse_uuid)
                    .transpose()?
                    .map(HostId),
                command: model.command,
                working_directory: model.working_directory,
                exit_code: model.exit_code,
                started_at_unix_secs: to_u64(model.started_at_unix_secs)?,
                duration_ms: model.duration_ms.map(to_u64).transpose()?,
            })
        })
        .collect()
}

async fn save_history(
    db: &DatabaseConnection,
    storage: &StorageManager,
) -> Result<(), StoragePersistenceError> {
    let now = current_unix_secs();
    for item in &storage.command_history {
        // command_history 已在 StorageManager 中做过数量限制和顺序维护。
        entity::command_history::Entity::insert(entity::command_history::ActiveModel {
            id: Set(item.id.0.to_string()),
            host_id: Set(item.host_id.map(|id| id.0.to_string())),
            command: Set(item.command.clone()),
            working_directory: Set(item.working_directory.clone()),
            exit_code: Set(item.exit_code),
            started_at_unix_secs: Set(item.started_at_unix_secs as i64),
            duration_ms: Set(item.duration_ms.map(|duration| duration as i64)),
            created_at_unix_secs: Set(now),
        })
        .exec(db)
        .await?;
    }

    for connection in &storage.recent_connections {
        // recent_connections 使用 host_id 做主键，保存的是每个主机最近一次连接。
        entity::recent_connection::Entity::insert(entity::recent_connection::ActiveModel {
            host_id: Set(connection.host_id.0.to_string()),
            label: Set(connection.label.clone()),
            connected_at_unix_secs: Set(connection.connected_at_unix_secs as i64),
            updated_at_unix_secs: Set(now),
        })
        .exec(db)
        .await?;
    }
    Ok(())
}

async fn load_snippets(db: &DatabaseConnection) -> Result<Vec<Snippet>, StoragePersistenceError> {
    // 变量和上次参数是 snippet 的子表，先按 snippet_id 分组再合成 Snippet。
    let mut variables_by_snippet: HashMap<String, Vec<entity::snippet_variable::Model>> =
        HashMap::new();
    for model in entity::snippet_variable::Entity::find()
        .order_by_asc(entity::snippet_variable::Column::SortOrder)
        .all(db)
        .await?
    {
        variables_by_snippet
            .entry(model.snippet_id.clone())
            .or_default()
            .push(model);
    }
    let mut arguments_by_snippet: HashMap<String, Vec<entity::snippet_argument::Model>> =
        HashMap::new();
    for model in entity::snippet_argument::Entity::find()
        .order_by_asc(entity::snippet_argument::Column::SortOrder)
        .all(db)
        .await?
    {
        arguments_by_snippet
            .entry(model.snippet_id.clone())
            .or_default()
            .push(model);
    }

    let mut snippets = Vec::new();
    for model in entity::snippet::Entity::find()
        .order_by_asc(entity::snippet::Column::SortOrder)
        .all(db)
        .await?
    {
        let variables = variables_by_snippet
            .remove(&model.id)
            .unwrap_or_default()
            .into_iter()
            .map(|variable| SnippetVariable {
                name: variable.name,
                default_value: variable.default_value,
                required: variable.required,
            })
            .collect();
        let last_arguments = arguments_by_snippet
            .remove(&model.id)
            .unwrap_or_default()
            .into_iter()
            .map(|argument| SnippetArgument {
                name: argument.name,
                value: argument.value,
            })
            .collect();

        snippets.push(Snippet {
            id: SnippetId(parse_uuid(&model.id)?),
            name: model.name,
            description: model.description,
            command_template: model.command_template,
            scope: snippet_scope_from_parts(&model.scope_kind, model.scope_target_id.as_deref())?,
            variables,
            last_arguments,
        });
    }
    Ok(snippets)
}

async fn save_snippets(
    db: &DatabaseConnection,
    snippets: &[Snippet],
) -> Result<(), StoragePersistenceError> {
    let now = current_unix_secs();
    for (index, snippet) in snippets.iter().enumerate() {
        let snippet_id = snippet.id.0.to_string();
        let (scope_kind, scope_target_id) = snippet_scope_to_parts(&snippet.scope);
        // scope 拆成 kind + target_id，便于后续按主机/分组查询。
        entity::snippet::Entity::insert(entity::snippet::ActiveModel {
            id: Set(snippet_id.clone()),
            name: Set(snippet.name.clone()),
            description: Set(snippet.description.clone()),
            command_template: Set(snippet.command_template.clone()),
            scope_kind: Set(scope_kind.to_owned()),
            scope_target_id: Set(scope_target_id),
            sort_order: Set(index as i32),
            created_at_unix_secs: Set(now),
            updated_at_unix_secs: Set(now),
        })
        .exec(db)
        .await?;

        for (variable_index, variable) in snippet.variables.iter().enumerate() {
            // 变量顺序按模板或用户编辑顺序保存。
            entity::snippet_variable::Entity::insert(entity::snippet_variable::ActiveModel {
                id: Set(format!("{snippet_id}:var:{variable_index}")),
                snippet_id: Set(snippet_id.clone()),
                name: Set(variable.name.clone()),
                default_value: Set(variable.default_value.clone()),
                required: Set(variable.required),
                sort_order: Set(variable_index as i32),
            })
            .exec(db)
            .await?;
        }

        for (argument_index, argument) in snippet.last_arguments.iter().enumerate() {
            // last_arguments 用于 UI 回填，不参与渲染规则判断。
            entity::snippet_argument::Entity::insert(entity::snippet_argument::ActiveModel {
                id: Set(format!("{snippet_id}:arg:{argument_index}")),
                snippet_id: Set(snippet_id.clone()),
                name: Set(argument.name.clone()),
                value: Set(argument.value.clone()),
                sort_order: Set(argument_index as i32),
            })
            .exec(db)
            .await?;
        }
    }
    Ok(())
}

async fn load_sftp_bookmarks(
    db: &DatabaseConnection,
) -> Result<Vec<SftpBookmark>, StoragePersistenceError> {
    entity::sftp_bookmark::Entity::find()
        .order_by_asc(entity::sftp_bookmark::Column::SortOrder)
        .all(db)
        .await?
        .into_iter()
        .map(|model| {
            Ok(SftpBookmark {
                host_id: HostId(parse_uuid(&model.host_id)?),
                label: model.label,
                remote_path: model.remote_path,
            })
        })
        .collect()
}

async fn save_sftp_bookmarks(
    db: &DatabaseConnection,
    bookmarks: &[SftpBookmark],
) -> Result<(), StoragePersistenceError> {
    let now = current_unix_secs();
    for (index, bookmark) in bookmarks.iter().enumerate() {
        // 同一主机同一路径只保留一个书签。
        entity::sftp_bookmark::Entity::insert(entity::sftp_bookmark::ActiveModel {
            id: Set(format!("{}:{}", bookmark.host_id.0, bookmark.remote_path)),
            host_id: Set(bookmark.host_id.0.to_string()),
            label: Set(bookmark.label.clone()),
            remote_path: Set(bookmark.remote_path.clone()),
            sort_order: Set(index as i32),
            created_at_unix_secs: Set(now),
            updated_at_unix_secs: Set(now),
        })
        .exec(db)
        .await?;
    }
    Ok(())
}

async fn load_tunnel_rules(
    db: &DatabaseConnection,
) -> Result<Vec<TunnelRule>, StoragePersistenceError> {
    entity::tunnel_rule::Entity::find()
        .order_by_asc(entity::tunnel_rule::Column::SortOrder)
        .all(db)
        .await?
        .into_iter()
        .map(|model| {
            Ok(TunnelRule {
                name: model.name,
                kind: tunnel_kind_from_str(&model.kind),
                bind_host: model.bind_host,
                bind_port: to_u16(model.bind_port)?,
                target_host: model.target_host,
                target_port: to_u16(model.target_port)?,
                auto_start: model.auto_start,
            })
        })
        .collect()
}

async fn save_tunnel_rules(
    db: &DatabaseConnection,
    rules: &[TunnelRule],
) -> Result<(), StoragePersistenceError> {
    let now = current_unix_secs();
    for (index, rule) in rules.iter().enumerate() {
        // 保存前规范化隧道规则，避免空格导致重复规则。
        let rule = rule.normalized();
        entity::tunnel_rule::Entity::insert(entity::tunnel_rule::ActiveModel {
            name: Set(rule.name),
            kind: Set(tunnel_kind_to_str(&rule.kind).to_owned()),
            bind_host: Set(rule.bind_host),
            bind_port: Set(rule.bind_port as i32),
            target_host: Set(rule.target_host),
            target_port: Set(rule.target_port as i32),
            auto_start: Set(rule.auto_start),
            sort_order: Set(index as i32),
            created_at_unix_secs: Set(now),
            updated_at_unix_secs: Set(now),
        })
        .exec(db)
        .await?;
    }
    Ok(())
}

async fn load_workspace(
    db: &DatabaseConnection,
) -> Result<Option<WorkspaceState>, StoragePersistenceError> {
    // 当前只保存一个默认工作区；多工作区时可以扩展 workspace_key。
    let Some(model) = entity::workspace_state::Entity::find()
        .filter(entity::workspace_state::Column::WorkspaceKey.eq(DEFAULT_WORKSPACE_KEY))
        .one(db)
        .await?
    else {
        return Ok(None);
    };

    Ok(Some(toml::from_str(&model.state_toml)?))
}

async fn save_workspace(
    db: &DatabaseConnection,
    workspace: Option<&WorkspaceState>,
) -> Result<(), StoragePersistenceError> {
    let Some(workspace) = workspace else {
        // 没有工作区快照时不写行，加载自然返回 None。
        return Ok(());
    };
    entity::workspace_state::Entity::insert(entity::workspace_state::ActiveModel {
        workspace_key: Set(DEFAULT_WORKSPACE_KEY.to_owned()),
        state_toml: Set(toml::to_string(workspace)?),
        updated_at_unix_secs: Set(current_unix_secs()),
    })
    .exec(db)
    .await?;
    Ok(())
}

fn auth_from_model(
    model: &entity::host_auth::Model,
) -> Result<AuthProfile, StoragePersistenceError> {
    // 对关键 SecretRef 字段使用 required_field，坏数据应 fail-fast 暴露出来。
    let profile = match model.auth_kind.as_str() {
        "password" => AuthProfile::Password {
            username: model.username.clone(),
            secret: SecretRef(required_field(
                model.password_secret_ref.clone(),
                "host_auth.password_secret_ref",
            )?),
        },
        "key" => AuthProfile::Key {
            username: model.username.clone(),
            key: SecretRef(required_field(
                model.key_secret_ref.clone(),
                "host_auth.key_secret_ref",
            )?),
            passphrase: model.passphrase_secret_ref.clone().map(SecretRef),
        },
        "agent" => AuthProfile::Agent {
            username: model.username.clone(),
            source: agent_source_from_parts(
                model.agent_source.as_deref(),
                model.agent_pipe.clone(),
            ),
            key_hint: model.key_hint.clone(),
        },
        "certificate" => AuthProfile::Certificate {
            username: model.username.clone(),
            key: SecretRef(required_field(
                model.key_secret_ref.clone(),
                "host_auth.key_secret_ref",
            )?),
            passphrase: model.passphrase_secret_ref.clone().map(SecretRef),
            certificate: SecretRef(required_field(
                model.certificate_secret_ref.clone(),
                "host_auth.certificate_secret_ref",
            )?),
        },
        _ => default_auth(),
    };
    Ok(profile)
}

fn proxy_from_model(
    model: &entity::host_proxy::Model,
) -> Result<ProxyProfile, StoragePersistenceError> {
    let port = to_u16(model.proxy_port)?;
    Ok(match model.proxy_kind.as_str() {
        "http" => ProxyProfile::Http {
            host: model.proxy_host.clone(),
            port,
        },
        _ => ProxyProfile::Socks5 {
            host: model.proxy_host.clone(),
            port,
        },
    })
}

fn default_auth() -> AuthProfile {
    // 缺失认证行时使用空用户名的 agent 认证，避免旧库加载崩溃；UI 保存后会写回完整配置。
    AuthProfile::Agent {
        username: String::new(),
        source: AgentSource::Auto,
        key_hint: None,
    }
}

fn agent_source_to_parts(source: &AgentSource) -> (String, Option<String>) {
    // 自定义 named pipe 需要额外保存 pipe，其余 agent source 只保存枚举 key。
    match source {
        AgentSource::Auto => ("auto".to_owned(), None),
        AgentSource::OpenSsh => ("openssh".to_owned(), None),
        AgentSource::Pageant => ("pageant".to_owned(), None),
        AgentSource::CustomNamedPipe(pipe) => ("custom_named_pipe".to_owned(), Some(pipe.clone())),
    }
}

fn agent_source_from_parts(source: Option<&str>, pipe: Option<String>) -> AgentSource {
    match source {
        Some("openssh") => AgentSource::OpenSsh,
        Some("pageant") => AgentSource::Pageant,
        Some("custom_named_pipe") => AgentSource::CustomNamedPipe(pipe.unwrap_or_default()),
        _ => AgentSource::Auto,
    }
}

fn credential_kind_to_str(kind: &CredentialKind) -> &'static str {
    match kind {
        CredentialKind::Password => "password",
        CredentialKind::PrivateKey => "private_key",
        CredentialKind::Agent => "agent",
        CredentialKind::Certificate => "certificate",
    }
}

fn credential_kind_from_str(kind: &str) -> CredentialKind {
    match kind {
        "private_key" => CredentialKind::PrivateKey,
        "agent" => CredentialKind::Agent,
        "certificate" => CredentialKind::Certificate,
        _ => CredentialKind::Password,
    }
}

fn key_algorithm_to_parts(algorithm: &KeyAlgorithm) -> (Option<String>, Option<String>) {
    match algorithm {
        KeyAlgorithm::Ed25519 => (Some("ed25519".to_owned()), None),
        KeyAlgorithm::Rsa => (Some("rsa".to_owned()), None),
        KeyAlgorithm::Ecdsa => (Some("ecdsa".to_owned()), None),
        KeyAlgorithm::Unknown(value) => (Some("unknown".to_owned()), Some(value.clone())),
    }
}

fn key_algorithm_from_parts(kind: &str, raw: Option<&str>) -> KeyAlgorithm {
    match kind {
        "ed25519" => KeyAlgorithm::Ed25519,
        "rsa" => KeyAlgorithm::Rsa,
        "ecdsa" => KeyAlgorithm::Ecdsa,
        _ => KeyAlgorithm::Unknown(raw.unwrap_or(kind).to_owned()),
    }
}

fn snippet_scope_to_parts(scope: &SnippetScope) -> (&'static str, Option<String>) {
    match scope {
        SnippetScope::Global => ("global", None),
        SnippetScope::Host(id) => ("host", Some(id.0.to_string())),
        SnippetScope::Group(id) => ("group", Some(id.0.to_string())),
    }
}

fn snippet_scope_from_parts(
    kind: &str,
    target_id: Option<&str>,
) -> Result<SnippetScope, StoragePersistenceError> {
    // host/group scope 必须带 target_id，缺失说明数据库数据不完整。
    Ok(match kind {
        "host" => SnippetScope::Host(HostId(parse_uuid(required_str(
            target_id,
            "snippets.scope_target_id",
        )?)?)),
        "group" => SnippetScope::Group(GroupId(parse_uuid(required_str(
            target_id,
            "snippets.scope_target_id",
        )?)?)),
        _ => SnippetScope::Global,
    })
}

fn tunnel_kind_to_str(kind: &TunnelKind) -> &'static str {
    match kind {
        TunnelKind::Local => "local",
        TunnelKind::Remote => "remote",
        TunnelKind::Dynamic => "dynamic",
    }
}

fn tunnel_kind_from_str(kind: &str) -> TunnelKind {
    match kind {
        "local" => TunnelKind::Local,
        "remote" => TunnelKind::Remote,
        _ => TunnelKind::Dynamic,
    }
}

fn encode_optional_toml<T: serde::Serialize>(
    value: Option<&T>,
) -> Result<Option<String>, StoragePersistenceError> {
    // 复杂扩展字段直接以 TOML 存储，避免 schema 为少量 override 过度膨胀。
    value.map(toml::to_string).transpose().map_err(Into::into)
}

fn decode_optional_toml<T: serde::de::DeserializeOwned>(
    value: Option<&str>,
) -> Result<Option<T>, StoragePersistenceError> {
    value.map(toml::from_str).transpose().map_err(Into::into)
}

fn parse_uuid(value: &str) -> Result<Uuid, StoragePersistenceError> {
    // 数据库中 UUID 全部以字符串保存，加载时统一校验格式。
    Uuid::parse_str(value).map_err(|error| StoragePersistenceError::InvalidData(error.to_string()))
}

fn to_u16(value: i32) -> Result<u16, StoragePersistenceError> {
    u16::try_from(value)
        .map_err(|_| StoragePersistenceError::InvalidData(format!("数值超出 u16 范围：{value}")))
}

fn to_u64(value: i64) -> Result<u64, StoragePersistenceError> {
    u64::try_from(value)
        .map_err(|_| StoragePersistenceError::InvalidData(format!("数值超出 u64 范围：{value}")))
}

fn required_field(
    value: Option<String>,
    field: &'static str,
) -> Result<String, StoragePersistenceError> {
    value.ok_or_else(|| StoragePersistenceError::InvalidData(format!("缺少字段：{field}")))
}

fn required_str<'a>(
    value: Option<&'a str>,
    field: &'static str,
) -> Result<&'a str, StoragePersistenceError> {
    value.ok_or_else(|| StoragePersistenceError::InvalidData(format!("缺少字段：{field}")))
}
