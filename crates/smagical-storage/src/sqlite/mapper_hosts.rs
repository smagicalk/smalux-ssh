use std::collections::HashMap;

use sea_orm::{ActiveValue::Set, DatabaseConnection, EntityTrait, QueryOrder};

use smagical_core::{
    AuthProfile, GroupId, Host, HostGroup, HostId, JumpProfile, ProxyProfile, SecretRef,
};

use super::mapper_common::*;
use super::{current_unix_secs, entity};
use crate::StoragePersistenceError;

pub(super) async fn load_groups(
    db: &DatabaseConnection,
) -> Result<Vec<HostGroup>, StoragePersistenceError> {
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

pub(super) async fn save_groups(
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

pub(super) async fn load_hosts(
    db: &DatabaseConnection,
) -> Result<Vec<Host>, StoragePersistenceError> {
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

pub(super) async fn save_hosts(
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
