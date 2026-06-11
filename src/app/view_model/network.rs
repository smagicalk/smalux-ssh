//! Network 页展示模型。

use crate::model::{AppState, ProxyProfile, TunnelRule, TunnelStatus};

use super::common::host_name;
use super::i18n::{locale_for_state, tr};
use super::tools_types::NetworkNavItemViewModel;

pub(in crate::app::view_model) fn runtime_tunnel_items(
    state: &AppState,
) -> Vec<NetworkNavItemViewModel> {
    let locale = locale_for_state(state);
    state
        .sessions
        .tunnels
        .iter()
        .map(|tunnel| {
            let endpoint = state
                .storage
                .tunnel_rules
                .iter()
                .find(|rule| rule.name == tunnel.rule_name)
                .map(TunnelRule::display_endpoint)
                .unwrap_or_else(|| tr(locale, "tool.tunnel_runtime").to_owned());
            let host_label = tunnel
                .host_id
                .map(|host_id| host_name(state, host_id))
                .unwrap_or_else(|| tr(locale, "common.unknown_host").to_owned());
            NetworkNavItemViewModel {
                id: format!("runtime:{}:{}", tunnel.session_id.0, tunnel.rule_name),
                title: tunnel.rule_name.clone(),
                subtitle: host_label.clone(),
                meta: tunnel_status_label(&tunnel.status, locale).to_owned(),
                kind_key: "TunnelRuntime",
                kind_label: tr(locale, "page.tunnels").to_owned(),
                note: tunnel
                    .last_error
                    .clone()
                    .unwrap_or_else(|| tr(locale, "tool.tunnel_runtime").to_owned()),
                icon_key: "router",
                accent_index: 0,
                session_id: tunnel.session_id.0.to_string(),
                primary_action_key: "stop",
                primary_action_label: tr(locale, "tool.known_hosts_delete").to_owned(),
                primary_action_enabled: matches!(
                    tunnel.status,
                    TunnelStatus::Starting | TunnelStatus::Running
                ),
                stat_primary_label: tr(locale, "proxy.section_forward").to_owned(),
                stat_primary_value: endpoint.clone(),
                stat_secondary_label: tr(locale, "tool.proxy").to_owned(),
                stat_secondary_value: tunnel_status_label(&tunnel.status, locale).to_owned(),
                detail_primary_label: tr(locale, "proxy.field_host").to_owned(),
                detail_primary_value: host_label,
                detail_secondary_label: tr(locale, "proxy.field_endpoint").to_owned(),
                detail_secondary_value: endpoint,
                body_label: tr(locale, "proxy.field_note").to_owned(),
                body_value: tunnel
                    .last_error
                    .clone()
                    .unwrap_or_else(|| tr(locale, "tool.tunnel_runtime").to_owned()),
                asset_id: String::new(),
                edit_kind_key: String::new(),
                edit_host: String::new(),
                edit_port: String::new(),
                edit_tags: String::new(),
                edit_bind_host: String::new(),
                edit_bind_port: String::new(),
                edit_target_host: String::new(),
                edit_target_port: String::new(),
                edit_auto_start: false,
                edit_host_ids: String::new(),
            }
        })
        .collect()
}

pub(in crate::app::view_model) fn network_resource_items(
    state: &AppState,
) -> Vec<NetworkNavItemViewModel> {
    let mut items = Vec::new();
    items.extend(proxy_asset_items(state));
    items.extend(jump_chain_asset_items(state));
    items.extend(forward_asset_items(state));
    items
}

fn proxy_asset_items(state: &AppState) -> Vec<NetworkNavItemViewModel> {
    let locale = locale_for_state(state);
    state
        .storage
        .proxy_assets
        .iter()
        .map(|asset| {
            let used_by = state
                .storage
                .proxy_asset_host_ids(asset.id)
                .into_iter()
                .map(|host_id| host_name(state, host_id))
                .collect::<Vec<_>>();
            NetworkNavItemViewModel {
                id: format!("proxy:{}", asset.id.0),
                title: asset.name.clone(),
                subtitle: proxy_profile_label(&asset.profile, locale),
                meta: network_tags_label(&asset.tags, locale),
                kind_key: "ProxyAsset",
                kind_label: tr(locale, "host.network_proxy_label").to_owned(),
                note: tr(locale, "proxy.resource_note").to_owned(),
                icon_key: "globe",
                accent_index: 1,
                session_id: String::new(),
                primary_action_key: "",
                primary_action_label: String::new(),
                primary_action_enabled: false,
                stat_primary_label: tr(locale, "proxy.field_type").to_owned(),
                stat_primary_value: proxy_protocol_label(&asset.profile, locale).to_owned(),
                stat_secondary_label: tr(locale, "proxy.field_usage").to_owned(),
                stat_secondary_value: used_by.len().to_string(),
                detail_primary_label: tr(locale, "proxy.field_address").to_owned(),
                detail_primary_value: proxy_endpoint_label(&asset.profile),
                detail_secondary_label: tr(locale, "proxy.field_tags").to_owned(),
                detail_secondary_value: network_tags_label(&asset.tags, locale),
                body_label: tr(locale, "proxy.field_used_by").to_owned(),
                body_value: used_by_label(&used_by, locale),
                asset_id: asset.id.0.to_string(),
                edit_kind_key: proxy_kind_key(&asset.profile).to_owned(),
                edit_host: proxy_host(&asset.profile).to_owned(),
                edit_port: proxy_port(&asset.profile).to_string(),
                edit_tags: asset.tags.join(", "),
                edit_bind_host: String::new(),
                edit_bind_port: String::new(),
                edit_target_host: String::new(),
                edit_target_port: String::new(),
                edit_auto_start: false,
                edit_host_ids: String::new(),
            }
        })
        .collect()
}

fn jump_chain_asset_items(state: &AppState) -> Vec<NetworkNavItemViewModel> {
    let locale = locale_for_state(state);
    state
        .storage
        .jump_chain_assets
        .iter()
        .map(|asset| {
            let hops = asset
                .steps
                .iter()
                .map(|step| host_name(state, step.host_id))
                .collect::<Vec<_>>();
            let used_by = state
                .storage
                .jump_chain_asset_host_ids(asset.id)
                .into_iter()
                .map(|host_id| host_name(state, host_id))
                .collect::<Vec<_>>();
            NetworkNavItemViewModel {
                id: format!("jump:{}", asset.id.0),
                title: asset.name.clone(),
                subtitle: if hops.is_empty() {
                    tr(locale, "tool.empty_value").to_owned()
                } else {
                    hops.join(" -> ")
                },
                meta: format!("{} {}", asset.steps.len(), tr(locale, "proxy.nodes_suffix")),
                kind_key: "JumpChainAsset",
                kind_label: tr(locale, "host.network_jump_label").to_owned(),
                note: tr(locale, "proxy.resource_note").to_owned(),
                icon_key: "router",
                accent_index: 2,
                session_id: String::new(),
                primary_action_key: "",
                primary_action_label: String::new(),
                primary_action_enabled: false,
                stat_primary_label: tr(locale, "proxy.field_nodes").to_owned(),
                stat_primary_value: asset.steps.len().to_string(),
                stat_secondary_label: tr(locale, "proxy.field_usage").to_owned(),
                stat_secondary_value: used_by.len().to_string(),
                detail_primary_label: tr(locale, "proxy.field_path").to_owned(),
                detail_primary_value: if hops.is_empty() {
                    tr(locale, "tool.empty_value").to_owned()
                } else {
                    hops.join(" -> ")
                },
                detail_secondary_label: tr(locale, "proxy.field_type").to_owned(),
                detail_secondary_value: tr(locale, "proxy.kind_jump_chain").to_owned(),
                body_label: tr(locale, "proxy.field_used_by").to_owned(),
                body_value: used_by_label(&used_by, locale),
                asset_id: asset.id.0.to_string(),
                edit_kind_key: "JumpChain".to_owned(),
                edit_host: String::new(),
                edit_port: String::new(),
                edit_tags: String::new(),
                edit_bind_host: String::new(),
                edit_bind_port: String::new(),
                edit_target_host: String::new(),
                edit_target_port: String::new(),
                edit_auto_start: false,
                edit_host_ids: asset
                    .steps
                    .iter()
                    .map(|step| step.host_id.0.to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
            }
        })
        .collect()
}

fn forward_asset_items(state: &AppState) -> Vec<NetworkNavItemViewModel> {
    let locale = locale_for_state(state);
    state
        .storage
        .forward_assets
        .iter()
        .map(|asset| {
            let used_by = state
                .storage
                .forward_asset_host_ids(asset.id)
                .into_iter()
                .map(|host_id| host_name(state, host_id))
                .collect::<Vec<_>>();
            NetworkNavItemViewModel {
                id: format!("forward:{}", asset.id.0),
                title: asset.name.clone(),
                subtitle: asset.rule.display_endpoint(),
                meta: network_tags_label(&asset.tags, locale),
                kind_key: "ForwardAsset",
                kind_label: tr(locale, "host.network_forward_label").to_owned(),
                note: tr(locale, "proxy.resource_note").to_owned(),
                icon_key: "router",
                accent_index: 3,
                session_id: String::new(),
                primary_action_key: "",
                primary_action_label: String::new(),
                primary_action_enabled: false,
                stat_primary_label: tr(locale, "proxy.field_type").to_owned(),
                stat_primary_value: tunnel_kind_label(&asset.rule, locale),
                stat_secondary_label: tr(locale, "proxy.field_usage").to_owned(),
                stat_secondary_value: used_by.len().to_string(),
                detail_primary_label: tr(locale, "proxy.field_bind").to_owned(),
                detail_primary_value: format!("{}:{}", asset.rule.bind_host, asset.rule.bind_port),
                detail_secondary_label: tr(locale, "proxy.field_target").to_owned(),
                detail_secondary_value: tunnel_target_label(&asset.rule, locale),
                body_label: tr(locale, "proxy.field_used_by").to_owned(),
                body_value: used_by_label(&used_by, locale),
                asset_id: asset.id.0.to_string(),
                edit_kind_key: tunnel_kind_key(&asset.rule).to_owned(),
                edit_host: String::new(),
                edit_port: String::new(),
                edit_tags: asset.tags.join(", "),
                edit_bind_host: asset.rule.bind_host.clone(),
                edit_bind_port: asset.rule.bind_port.to_string(),
                edit_target_host: asset.rule.target_host.clone(),
                edit_target_port: if asset.rule.target_port == 0 {
                    String::new()
                } else {
                    asset.rule.target_port.to_string()
                },
                edit_auto_start: asset.rule.auto_start,
                edit_host_ids: String::new(),
            }
        })
        .collect()
}

fn proxy_profile_label(profile: &ProxyProfile, locale: super::i18n::Locale) -> String {
    match profile {
        ProxyProfile::Socks5 { host, port } => {
            format!("{} {host}:{port}", tr(locale, "proxy.kind_socks5"))
        }
        ProxyProfile::Http { host, port } => {
            format!("{} {host}:{port}", tr(locale, "proxy.kind_http"))
        }
    }
}

fn proxy_protocol_label(profile: &ProxyProfile, locale: super::i18n::Locale) -> &'static str {
    match profile {
        ProxyProfile::Socks5 { .. } => tr(locale, "proxy.kind_socks5"),
        ProxyProfile::Http { .. } => tr(locale, "proxy.kind_http"),
    }
}

fn proxy_kind_key(profile: &ProxyProfile) -> &'static str {
    match profile {
        ProxyProfile::Socks5 { .. } => "Socks5",
        ProxyProfile::Http { .. } => "Http",
    }
}

fn proxy_host(profile: &ProxyProfile) -> &str {
    match profile {
        ProxyProfile::Socks5 { host, .. } | ProxyProfile::Http { host, .. } => host,
    }
}

fn proxy_port(profile: &ProxyProfile) -> u16 {
    match profile {
        ProxyProfile::Socks5 { port, .. } | ProxyProfile::Http { port, .. } => *port,
    }
}

fn proxy_endpoint_label(profile: &ProxyProfile) -> String {
    match profile {
        ProxyProfile::Socks5 { host, port } | ProxyProfile::Http { host, port } => {
            format!("{host}:{port}")
        }
    }
}

fn network_tags_label(tags: &[String], locale: super::i18n::Locale) -> String {
    if tags.is_empty() {
        tr(locale, "common.untagged").to_owned()
    } else {
        tags.join(" / ")
    }
}

fn used_by_label(host_names: &[String], locale: super::i18n::Locale) -> String {
    if host_names.is_empty() {
        tr(locale, "proxy.used_by_empty").to_owned()
    } else {
        format!(
            "{} {}: {}",
            host_names.len(),
            tr(locale, "proxy.used_by_hosts_suffix"),
            host_names.join(" / ")
        )
    }
}

fn tunnel_target_label(rule: &TunnelRule, locale: super::i18n::Locale) -> String {
    if matches!(rule.kind, crate::model::TunnelKind::Dynamic) {
        tr(locale, "proxy.dynamic_runtime_target").to_owned()
    } else {
        format!("{}:{}", rule.target_host, rule.target_port)
    }
}

fn tunnel_kind_label(rule: &TunnelRule, locale: super::i18n::Locale) -> String {
    match rule.kind {
        crate::model::TunnelKind::Local => tr(locale, "host.network_forward_local").to_owned(),
        crate::model::TunnelKind::Remote => tr(locale, "host.network_forward_remote").to_owned(),
        crate::model::TunnelKind::Dynamic => tr(locale, "host.network_forward_dynamic").to_owned(),
    }
}

fn tunnel_kind_key(rule: &TunnelRule) -> &'static str {
    match rule.kind {
        crate::model::TunnelKind::Local => "Local",
        crate::model::TunnelKind::Remote => "Remote",
        crate::model::TunnelKind::Dynamic => "Dynamic",
    }
}

fn tunnel_status_label(status: &TunnelStatus, locale: super::i18n::Locale) -> &'static str {
    match status {
        TunnelStatus::Stopped => tr(locale, "tool.tunnel_status_stopped"),
        TunnelStatus::Starting => tr(locale, "tool.tunnel_status_starting"),
        TunnelStatus::Running => tr(locale, "tool.tunnel_status_running"),
        TunnelStatus::Stopping => tr(locale, "tool.tunnel_status_stopping"),
        TunnelStatus::Failed => tr(locale, "tool.tunnel_status_failed"),
    }
}
