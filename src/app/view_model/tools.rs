//! 右侧工具分栏展示模型。

use crate::model::AppState;

/// 右侧工具分栏的通用列表项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct ToolItemViewModel {
    pub title: String,
    pub subtitle: String,
    pub meta: String,
}

/// Known Hosts 工具分栏的展示项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct KnownHostViewModel {
    pub host: String,
    pub port: u16,
    pub fingerprint: String,
    pub status: String,
}

pub(super) fn snippet_items(state: &AppState) -> Vec<ToolItemViewModel> {
    state
        .storage
        .snippets
        .iter()
        .map(|snippet| ToolItemViewModel {
            title: snippet.name.clone(),
            subtitle: snippet
                .description
                .clone()
                .unwrap_or_else(|| snippet.command_template.clone()),
            meta: format!("{} vars", snippet.variables.len()),
        })
        .collect()
}

pub(super) fn tunnel_items(state: &AppState) -> Vec<ToolItemViewModel> {
    let saved = state
        .storage
        .tunnel_rules
        .iter()
        .map(|rule| ToolItemViewModel {
            title: rule.name.clone(),
            subtitle: rule.display_endpoint(),
            meta: if rule.auto_start { "auto" } else { "saved" }.to_owned(),
        });
    let runtime = state
        .sessions
        .tunnels
        .iter()
        .map(|tunnel| ToolItemViewModel {
            title: tunnel.rule_name.clone(),
            subtitle: tunnel
                .last_error
                .clone()
                .unwrap_or_else(|| "runtime".to_owned()),
            meta: format!("{:?}", tunnel.status),
        });

    saved.chain(runtime).collect()
}

pub(super) fn known_host_items(state: &AppState) -> Vec<KnownHostViewModel> {
    state
        .storage
        .known_hosts
        .iter()
        .map(|entry| KnownHostViewModel {
            host: entry.host.clone(),
            port: entry.port,
            fingerprint: entry.fingerprint.clone(),
            status: if entry.trusted { "trusted" } else { "pending" }.to_owned(),
        })
        .collect()
}
