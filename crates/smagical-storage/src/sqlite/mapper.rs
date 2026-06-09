//! `StorageManager` 与 SQLite 业务表之间的映射。
//!
//! `StorageManager` 是应用内部的内存快照，SQLite 是落盘结构。这个模块是两者之间唯一的
//! 编解码层：加载时把多表数据聚合成内存结构，保存时把内存结构拆回多张业务表。

use std::collections::HashMap;

use super::mapper_common::*;
use super::mapper_credentials::{
    load_credential_groups, load_credential_inspections, load_credentials, load_secrets,
    save_credential_groups, save_credential_inspections, save_credentials, save_secrets,
};
use super::mapper_hosts::{load_groups, load_hosts, save_groups, save_hosts};
use super::{
    APP_CONFIG_SETTING_KEY, DEFAULT_WORKSPACE_KEY, clear_entities, current_unix_secs, entity,
};
use crate::{StorageManager, StoragePersistenceError, ThemeProfileRecord};
use sea_orm::{
    ActiveValue::Set, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
};
use smagical_config::AppConfig;
use smagical_core::{
    CommandHistoryId, CommandHistoryItem, HostId, KnownHostEntry, RecentConnection, SftpBookmark,
    Snippet, SnippetArgument, SnippetGroup, SnippetGroupId, SnippetId, SnippetImplementation,
    SnippetImplementationId, SnippetSupportTarget, SnippetSupportTargetId, SnippetVariable,
    TunnelRule, WorkspaceState,
};

pub(super) async fn load_storage(
    db: &DatabaseConnection,
) -> Result<StorageManager, StoragePersistenceError> {
    // 加载顺序先读独立数据，再通过 StorageManager 方法导入有不变量的集合。
    let mut storage = StorageManager::default();
    storage.app_config = load_app_config(db).await?;
    storage.groups = load_groups(db).await?;
    storage.hosts = load_hosts(db).await?;
    storage.credential_groups = load_credential_groups(db).await?;
    storage.credentials = load_credentials(db).await?;
    storage.credential_inspections = load_credential_inspections(db).await?;
    storage.secrets = load_secrets(db).await?;
    storage.known_hosts = load_known_hosts(db).await?;

    for connection in load_recent_connections(db).await? {
        storage.record_recent_connection(connection);
    }
    for item in load_command_history(db).await? {
        storage.add_command_history(item);
    }

    storage.snippet_groups = load_snippet_groups(db).await?;
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
    save_credential_groups(db, &storage.credential_groups).await?;
    save_credentials(db, &storage.credentials).await?;
    save_credential_inspections(db, &storage.credential_inspections).await?;
    save_secrets(db, &storage.secrets).await?;
    save_known_hosts(db, &storage.known_hosts).await?;
    save_history(db, storage).await?;
    save_snippet_groups(db, &storage.snippet_groups).await?;
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
    clear_entities::<entity::snippet_support_target::Entity>(db).await?;
    clear_entities::<entity::snippet_implementation::Entity>(db).await?;
    clear_entities::<entity::snippet_variable::Entity>(db).await?;
    clear_entities::<entity::snippet::Entity>(db).await?;
    clear_entities::<entity::snippet_group::Entity>(db).await?;
    clear_entities::<entity::recent_connection::Entity>(db).await?;
    clear_entities::<entity::command_history::Entity>(db).await?;
    clear_entities::<entity::known_host::Entity>(db).await?;
    clear_entities::<entity::credential_inspection::Entity>(db).await?;
    clear_entities::<entity::secret::Entity>(db).await?;
    clear_entities::<entity::credential::Entity>(db).await?;
    clear_entities::<entity::credential_group::Entity>(db).await?;
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
    // 变量、实现、支持目标和上次参数都是 snippet 的子表，先分组再合成 Snippet。
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
    let mut arguments_by_implementation: HashMap<String, Vec<entity::snippet_argument::Model>> =
        HashMap::new();
    for model in entity::snippet_argument::Entity::find()
        .order_by_asc(entity::snippet_argument::Column::SortOrder)
        .all(db)
        .await?
    {
        arguments_by_implementation
            .entry(model.implementation_id.clone())
            .or_default()
            .push(model);
    }
    let mut implementations_by_snippet: HashMap<
        String,
        Vec<entity::snippet_implementation::Model>,
    > = HashMap::new();
    for model in entity::snippet_implementation::Entity::find()
        .order_by_asc(entity::snippet_implementation::Column::SortOrder)
        .all(db)
        .await?
    {
        implementations_by_snippet
            .entry(model.snippet_id.clone())
            .or_default()
            .push(model);
    }
    let mut support_targets_by_snippet: HashMap<
        String,
        Vec<entity::snippet_support_target::Model>,
    > = HashMap::new();
    for model in entity::snippet_support_target::Entity::find()
        .order_by_asc(entity::snippet_support_target::Column::SortOrder)
        .all(db)
        .await?
    {
        support_targets_by_snippet
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

        let snippet_id = SnippetId(parse_uuid(&model.id)?);
        let mut implementations = implementations_by_snippet
            .remove(&model.id)
            .unwrap_or_default()
            .into_iter()
            .map(|implementation| {
                let implementation_id = SnippetImplementationId(parse_uuid(&implementation.id)?);
                let last_arguments = arguments_by_implementation
                    .remove(&implementation.id)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|argument| SnippetArgument {
                        name: argument.name,
                        value: argument.value,
                    })
                    .collect();
                Ok(SnippetImplementation {
                    id: implementation_id,
                    snippet_id,
                    name: implementation.name,
                    shell: snippet_shell_from_parts(
                        implementation.shell.as_str(),
                        implementation.shell_custom,
                    ),
                    command_template: implementation.command_template,
                    notes: implementation.notes,
                    last_arguments,
                    sort_order: implementation.sort_order,
                })
            })
            .collect::<Result<Vec<_>, StoragePersistenceError>>()?;
        let mut support_targets = support_targets_by_snippet
            .remove(&model.id)
            .unwrap_or_default()
            .into_iter()
            .map(|target| {
                Ok(SnippetSupportTarget {
                    id: SnippetSupportTargetId(parse_uuid(&target.id)?),
                    snippet_id,
                    target_key: target.target_key,
                    display_name: target.display_name,
                    implementation_id: SnippetImplementationId(parse_uuid(
                        &target.implementation_id,
                    )?),
                    sort_order: target.sort_order,
                })
            })
            .collect::<Result<Vec<_>, StoragePersistenceError>>()?;

        // 开发期旧库可能还没有新子表；用旧 command_template 构造默认实现，保证可加载后再保存为新结构。
        if implementations.is_empty() {
            let fallback = Snippet::with_default_implementation(
                snippet_id,
                model.name.clone(),
                model.description.clone(),
                snippet_scope_from_parts(&model.scope_kind, model.scope_target_id.as_deref())?,
                model
                    .group_id
                    .as_deref()
                    .map(parse_uuid)
                    .transpose()?
                    .map(SnippetGroupId),
                model.command_template.clone(),
            );
            implementations = fallback.implementations;
            support_targets = fallback.support_targets;
        }
        snippets.push(Snippet {
            id: snippet_id,
            name: model.name,
            description: model.description,
            scope: snippet_scope_from_parts(&model.scope_kind, model.scope_target_id.as_deref())?,
            group_id: model
                .group_id
                .as_deref()
                .map(parse_uuid)
                .transpose()?
                .map(SnippetGroupId),
            variables,
            implementations,
            support_targets,
        });
    }
    Ok(snippets)
}

async fn load_snippet_groups(
    db: &DatabaseConnection,
) -> Result<Vec<SnippetGroup>, StoragePersistenceError> {
    entity::snippet_group::Entity::find()
        .order_by_asc(entity::snippet_group::Column::SortOrder)
        .order_by_asc(entity::snippet_group::Column::Name)
        .all(db)
        .await?
        .into_iter()
        .map(|model| {
            Ok(SnippetGroup {
                id: SnippetGroupId(parse_uuid(&model.id)?),
                name: model.name,
                parent_id: model
                    .parent_id
                    .as_deref()
                    .map(parse_uuid)
                    .transpose()?
                    .map(SnippetGroupId),
                sort_order: model.sort_order,
            })
        })
        .collect()
}

async fn save_snippet_groups(
    db: &DatabaseConnection,
    groups: &[SnippetGroup],
) -> Result<(), StoragePersistenceError> {
    let now = current_unix_secs();
    for (index, group) in groups.iter().enumerate() {
        entity::snippet_group::Entity::insert(entity::snippet_group::ActiveModel {
            id: Set(group.id.0.to_string()),
            name: Set(group.name.clone()),
            parent_id: Set(group.parent_id.map(|id| id.0.to_string())),
            sort_order: Set(group.sort_order.max(index as i32)),
            created_at_unix_secs: Set(now),
            updated_at_unix_secs: Set(now),
        })
        .exec(db)
        .await?;
    }
    Ok(())
}

async fn save_snippets(
    db: &DatabaseConnection,
    snippets: &[Snippet],
) -> Result<(), StoragePersistenceError> {
    let now = current_unix_secs();
    for (index, snippet) in snippets.iter().enumerate() {
        let snippet_id = snippet.id.0.to_string();
        let (scope_kind, scope_target_id) = snippet_scope_to_parts(&snippet.scope);
        // scope 拆成 kind + target_id，便于后续按主机查询。
        entity::snippet::Entity::insert(entity::snippet::ActiveModel {
            id: Set(snippet_id.clone()),
            name: Set(snippet.name.clone()),
            description: Set(snippet.description.clone()),
            // 旧字段保留给历史迁移兜底；正式内容保存在 snippet_implementations。
            command_template: Set(snippet.default_command_template().to_owned()),
            scope_kind: Set(scope_kind.to_owned()),
            scope_target_id: Set(scope_target_id),
            group_id: Set(snippet.group_id.map(|id| id.0.to_string())),
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

        for (implementation_index, implementation) in snippet.implementations.iter().enumerate() {
            let implementation_id = implementation.id.0.to_string();
            let (shell, shell_custom) = snippet_shell_to_parts(&implementation.shell);
            entity::snippet_implementation::Entity::insert(
                entity::snippet_implementation::ActiveModel {
                    id: Set(implementation_id.clone()),
                    snippet_id: Set(snippet_id.clone()),
                    name: Set(implementation.name.clone()),
                    shell: Set(shell.to_owned()),
                    shell_custom: Set(shell_custom),
                    command_template: Set(implementation.command_template.clone()),
                    notes: Set(implementation.notes.clone()),
                    sort_order: Set(implementation.sort_order.max(implementation_index as i32)),
                    created_at_unix_secs: Set(now),
                    updated_at_unix_secs: Set(now),
                },
            )
            .exec(db)
            .await?;

            for (argument_index, argument) in implementation.last_arguments.iter().enumerate() {
                // last_arguments 用于 UI 回填，不参与渲染规则判断。
                entity::snippet_argument::Entity::insert(entity::snippet_argument::ActiveModel {
                    id: Set(format!("{implementation_id}:arg:{argument_index}")),
                    snippet_id: Set(Some(snippet_id.clone())),
                    implementation_id: Set(implementation_id.clone()),
                    name: Set(argument.name.clone()),
                    value: Set(argument.value.clone()),
                    sort_order: Set(argument_index as i32),
                })
                .exec(db)
                .await?;
            }
        }

        for (target_index, target) in snippet.support_targets.iter().enumerate() {
            entity::snippet_support_target::Entity::insert(
                entity::snippet_support_target::ActiveModel {
                    id: Set(target.id.0.to_string()),
                    snippet_id: Set(snippet_id.clone()),
                    target_key: Set(target.target_key.clone()),
                    display_name: Set(target.display_name.clone()),
                    implementation_id: Set(target.implementation_id.0.to_string()),
                    sort_order: Set(target.sort_order.max(target_index as i32)),
                    created_at_unix_secs: Set(now),
                    updated_at_unix_secs: Set(now),
                },
            )
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
