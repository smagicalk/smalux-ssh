use std::collections::HashMap;

use sea_orm::{ActiveValue::Set, DatabaseConnection, EntityTrait, QueryOrder};

use smagical_core::{
    ForwardAsset, ForwardId, HostId, JumpChainAsset, JumpChainId, JumpProfile, ProxyAsset,
    ProxyAuth, ProxyId, ProxyProfile, SecretRef, TunnelKind, TunnelRule,
};

use super::entity;
use super::mapper_common::*;
use crate::StoragePersistenceError;

pub(super) async fn load_proxy_assets(
    db: &DatabaseConnection,
) -> Result<Vec<ProxyAsset>, StoragePersistenceError> {
    entity::proxy_asset::Entity::find()
        .order_by_asc(entity::proxy_asset::Column::SortOrder)
        .all(db)
        .await?
        .into_iter()
        .map(|model| {
            Ok(ProxyAsset {
                id: ProxyId(parse_uuid(&model.id)?),
                name: model.name,
                tags: decode_string_list_toml(&model.tags_toml)?,
                profile: proxy_parts_to_profile(
                    &model.proxy_kind,
                    &model.proxy_host,
                    model.proxy_port,
                    &model.auth_kind,
                    model.auth_username,
                    model.auth_password_secret_ref,
                    model.remote_dns,
                )?,
            })
        })
        .collect()
}

pub(super) async fn save_proxy_assets(
    db: &DatabaseConnection,
    assets: &[ProxyAsset],
) -> Result<(), StoragePersistenceError> {
    for (index, asset) in assets.iter().enumerate() {
        let parts = proxy_profile_to_parts(&asset.profile);
        entity::proxy_asset::Entity::insert(entity::proxy_asset::ActiveModel {
            id: Set(asset.id.0.to_string()),
            name: Set(asset.name.clone()),
            tags_toml: Set(encode_string_list_toml(&asset.tags)?),
            proxy_kind: Set(parts.kind.to_owned()),
            proxy_host: Set(parts.host),
            proxy_port: Set(parts.port as i32),
            auth_kind: Set(parts.auth_kind.to_owned()),
            auth_username: Set(parts.auth_username),
            auth_password_secret_ref: Set(parts.auth_password_secret_ref),
            remote_dns: Set(parts.remote_dns),
            sort_order: Set(index as i32),
        })
        .exec(db)
        .await?;
    }
    Ok(())
}

pub(super) async fn load_jump_chain_assets(
    db: &DatabaseConnection,
) -> Result<Vec<JumpChainAsset>, StoragePersistenceError> {
    let mut steps_by_chain: HashMap<String, Vec<entity::jump_chain_step::Model>> = HashMap::new();
    for model in entity::jump_chain_step::Entity::find()
        .order_by_asc(entity::jump_chain_step::Column::SortOrder)
        .all(db)
        .await?
    {
        steps_by_chain
            .entry(model.chain_id.clone())
            .or_default()
            .push(model);
    }

    entity::jump_chain_asset::Entity::find()
        .order_by_asc(entity::jump_chain_asset::Column::SortOrder)
        .all(db)
        .await?
        .into_iter()
        .map(|model| {
            let steps = steps_by_chain
                .remove(&model.id)
                .unwrap_or_default()
                .into_iter()
                .map(|step| {
                    Ok(JumpProfile {
                        host_id: HostId(parse_uuid(&step.jump_host_id)?),
                        username_override: step.username_override,
                        port_override: step.port_override.map(to_u16).transpose()?,
                        alias: step.alias,
                    })
                })
                .collect::<Result<Vec<_>, StoragePersistenceError>>()?;
            Ok(JumpChainAsset {
                id: JumpChainId(parse_uuid(&model.id)?),
                name: model.name,
                steps,
                stop_on_failure: true,
            })
        })
        .collect()
}

pub(super) async fn save_jump_chain_assets(
    db: &DatabaseConnection,
    assets: &[JumpChainAsset],
) -> Result<(), StoragePersistenceError> {
    for (index, asset) in assets.iter().enumerate() {
        let chain_id = asset.id.0.to_string();
        entity::jump_chain_asset::Entity::insert(entity::jump_chain_asset::ActiveModel {
            id: Set(chain_id.clone()),
            name: Set(asset.name.clone()),
            sort_order: Set(index as i32),
        })
        .exec(db)
        .await?;

        for (step_index, step) in asset.steps.iter().enumerate() {
            entity::jump_chain_step::Entity::insert(entity::jump_chain_step::ActiveModel {
                id: Set(format!("{chain_id}:step:{step_index}")),
                chain_id: Set(chain_id.clone()),
                jump_host_id: Set(step.host_id.0.to_string()),
                username_override: Set(step.username_override.clone()),
                port_override: Set(step.port_override.map(|port| port as i32)),
                alias: Set(step.alias.clone()),
                sort_order: Set(step_index as i32),
            })
            .exec(db)
            .await?;
        }
    }
    Ok(())
}

pub(super) async fn load_forward_assets(
    db: &DatabaseConnection,
) -> Result<Vec<ForwardAsset>, StoragePersistenceError> {
    entity::forward_asset::Entity::find()
        .order_by_asc(entity::forward_asset::Column::SortOrder)
        .all(db)
        .await?
        .into_iter()
        .map(|model| {
            Ok(ForwardAsset {
                id: ForwardId(parse_uuid(&model.id)?),
                name: model.name.clone(),
                tags: decode_string_list_toml(&model.tags_toml)?,
                rule: TunnelRule {
                    name: model.name,
                    kind: tunnel_kind_from_str(&model.kind),
                    bind_host: model.bind_host,
                    bind_port: to_u16(model.bind_port)?,
                    target_host: model.target_host,
                    target_port: to_u16(model.target_port)?,
                    auto_start: model.auto_start,
                    exit_on_failure: model.exit_on_failure,
                },
                exit_on_failure: model.exit_on_failure,
            })
        })
        .collect()
}

pub(super) async fn save_forward_assets(
    db: &DatabaseConnection,
    assets: &[ForwardAsset],
) -> Result<(), StoragePersistenceError> {
    for (index, asset) in assets.iter().enumerate() {
        let rule = asset.rule.normalized();
        entity::forward_asset::Entity::insert(entity::forward_asset::ActiveModel {
            id: Set(asset.id.0.to_string()),
            name: Set(asset.name.clone()),
            tags_toml: Set(encode_string_list_toml(&asset.tags)?),
            kind: Set(tunnel_kind_to_str(&rule.kind).to_owned()),
            bind_host: Set(rule.bind_host),
            bind_port: Set(rule.bind_port as i32),
            target_host: Set(rule.target_host),
            target_port: Set(rule.target_port as i32),
            auto_start: Set(rule.auto_start),
            exit_on_failure: Set(asset.exit_on_failure),
            sort_order: Set(index as i32),
        })
        .exec(db)
        .await?;
    }
    Ok(())
}

pub(super) struct ProxyProfileParts {
    pub(super) kind: &'static str,
    pub(super) host: String,
    pub(super) port: u16,
    pub(super) auth_kind: &'static str,
    pub(super) auth_username: Option<String>,
    pub(super) auth_password_secret_ref: Option<String>,
    pub(super) remote_dns: bool,
}

pub(super) fn proxy_profile_to_parts(proxy: &ProxyProfile) -> ProxyProfileParts {
    match proxy {
        ProxyProfile::Socks5 {
            host,
            port,
            auth,
            remote_dns,
        } => proxy_parts("socks5", host, *port, auth, *remote_dns),
        ProxyProfile::Http { host, port, auth } => proxy_parts("http", host, *port, auth, false),
    }
}

pub(super) fn proxy_parts_to_profile(
    proxy_kind: &str,
    proxy_host: &str,
    proxy_port: i32,
    auth_kind: &str,
    auth_username: Option<String>,
    auth_password_secret_ref: Option<String>,
    remote_dns: bool,
) -> Result<ProxyProfile, StoragePersistenceError> {
    let port = to_u16(proxy_port)?;
    let auth = proxy_auth_from_parts(auth_kind, auth_username, auth_password_secret_ref);
    Ok(match proxy_kind {
        "http" => ProxyProfile::Http {
            host: proxy_host.to_owned(),
            port,
            auth,
        },
        _ => ProxyProfile::Socks5 {
            host: proxy_host.to_owned(),
            port,
            auth,
            remote_dns,
        },
    })
}

fn proxy_parts(
    kind: &'static str,
    host: &str,
    port: u16,
    auth: &ProxyAuth,
    remote_dns: bool,
) -> ProxyProfileParts {
    let (auth_kind, auth_username, auth_password_secret_ref) = proxy_auth_to_parts(auth);
    ProxyProfileParts {
        kind,
        host: host.to_owned(),
        port,
        auth_kind,
        auth_username,
        auth_password_secret_ref,
        remote_dns,
    }
}

fn proxy_auth_to_parts(auth: &ProxyAuth) -> (&'static str, Option<String>, Option<String>) {
    match auth {
        ProxyAuth::None => ("none", None, None),
        ProxyAuth::UserPassword { username, password } => (
            "user_password",
            Some(username.clone()),
            password.as_ref().map(|secret| secret.0.clone()),
        ),
    }
}

fn proxy_auth_from_parts(
    auth_kind: &str,
    auth_username: Option<String>,
    auth_password_secret_ref: Option<String>,
) -> ProxyAuth {
    match auth_kind {
        "user_password" | "basic" => ProxyAuth::UserPassword {
            username: auth_username.unwrap_or_default(),
            password: auth_password_secret_ref.map(SecretRef),
        },
        _ => ProxyAuth::None,
    }
}

pub(super) fn tunnel_kind_from_str(kind: &str) -> TunnelKind {
    match kind {
        "remote" => TunnelKind::Remote,
        "dynamic" => TunnelKind::Dynamic,
        _ => TunnelKind::Local,
    }
}

pub(super) fn tunnel_kind_to_str(kind: &TunnelKind) -> &'static str {
    match kind {
        TunnelKind::Local => "local",
        TunnelKind::Remote => "remote",
        TunnelKind::Dynamic => "dynamic",
    }
}
