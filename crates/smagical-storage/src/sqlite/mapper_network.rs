use std::collections::HashMap;

use sea_orm::{ActiveValue::Set, DatabaseConnection, EntityTrait, QueryOrder};

use smagical_core::{
    ForwardAsset, ForwardId, HostId, JumpChainAsset, JumpChainId, JumpProfile, ProxyAsset, ProxyId,
    ProxyProfile, TunnelKind, TunnelRule,
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
        let (proxy_kind, proxy_host, proxy_port) = proxy_profile_to_parts(&asset.profile);
        entity::proxy_asset::Entity::insert(entity::proxy_asset::ActiveModel {
            id: Set(asset.id.0.to_string()),
            name: Set(asset.name.clone()),
            tags_toml: Set(encode_string_list_toml(&asset.tags)?),
            proxy_kind: Set(proxy_kind.to_owned()),
            proxy_host: Set(proxy_host.clone()),
            proxy_port: Set(proxy_port as i32),
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
                    })
                })
                .collect::<Result<Vec<_>, StoragePersistenceError>>()?;
            Ok(JumpChainAsset {
                id: JumpChainId(parse_uuid(&model.id)?),
                name: model.name,
                steps,
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
                },
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
            sort_order: Set(index as i32),
        })
        .exec(db)
        .await?;
    }
    Ok(())
}

pub(super) fn proxy_profile_to_parts(proxy: &ProxyProfile) -> (&'static str, String, u16) {
    match proxy {
        ProxyProfile::Socks5 { host, port } => ("socks5", host.clone(), *port),
        ProxyProfile::Http { host, port } => ("http", host.clone(), *port),
    }
}

pub(super) fn proxy_parts_to_profile(
    proxy_kind: &str,
    proxy_host: &str,
    proxy_port: i32,
) -> Result<ProxyProfile, StoragePersistenceError> {
    let port = to_u16(proxy_port)?;
    Ok(match proxy_kind {
        "http" => ProxyProfile::Http {
            host: proxy_host.to_owned(),
            port,
        },
        _ => ProxyProfile::Socks5 {
            host: proxy_host.to_owned(),
            port,
        },
    })
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
