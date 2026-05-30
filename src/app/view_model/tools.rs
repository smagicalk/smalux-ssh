//! 右侧工具分栏展示模型。

use crate::model::{AppState, CredentialKind, KeyAlgorithm, TunnelStatus};

use super::i18n::{locale_for_state, tr};

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
    pub status_key: String,
    pub status: String,
}

pub(super) fn snippet_items(state: &AppState) -> Vec<ToolItemViewModel> {
    let locale = locale_for_state(state);
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
            meta: format!(
                "{}{}",
                snippet.variables.len(),
                tr(locale, "tool.snippet_vars_suffix")
            ),
        })
        .collect()
}

pub(super) fn tunnel_items(state: &AppState) -> Vec<ToolItemViewModel> {
    let locale = locale_for_state(state);
    let saved = state
        .storage
        .tunnel_rules
        .iter()
        .map(|rule| ToolItemViewModel {
            title: rule.name.clone(),
            subtitle: rule.display_endpoint(),
            meta: if rule.auto_start {
                tr(locale, "tool.tunnel_auto")
            } else {
                tr(locale, "tool.tunnel_saved")
            }
            .to_owned(),
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
                .unwrap_or_else(|| tr(locale, "tool.tunnel_runtime").to_owned()),
            meta: tunnel_status_label(&tunnel.status, locale).to_owned(),
        });

    saved.chain(runtime).collect()
}

pub(super) fn credential_items(state: &AppState) -> Vec<ToolItemViewModel> {
    let locale = locale_for_state(state);
    state
        .storage
        .credentials
        .iter()
        .map(|credential| ToolItemViewModel {
            title: credential.name.clone(),
            subtitle: credential
                .username
                .clone()
                .unwrap_or_else(|| credential_kind_label(&credential.kind, locale).to_owned()),
            meta: credential
                .fingerprint
                .clone()
                .or_else(|| credential.key_algorithm.as_ref().map(key_algorithm_label))
                .unwrap_or_else(|| credential_kind_label(&credential.kind, locale).to_owned()),
        })
        .collect()
}

pub(super) fn known_host_items(state: &AppState) -> Vec<KnownHostViewModel> {
    let locale = locale_for_state(state);
    state
        .storage
        .known_hosts
        .iter()
        .map(|entry| KnownHostViewModel {
            host: entry.host.clone(),
            port: entry.port,
            fingerprint: entry.fingerprint.clone(),
            status_key: if entry.trusted { "trusted" } else { "pending" }.to_owned(),
            status: if entry.trusted {
                tr(locale, "tool.known_host_trusted")
            } else {
                tr(locale, "tool.known_host_pending")
            }
            .to_owned(),
        })
        .collect()
}

fn credential_kind_label(kind: &CredentialKind, locale: super::i18n::Locale) -> &'static str {
    match kind {
        CredentialKind::Password => tr(locale, "tool.credential_password"),
        CredentialKind::PrivateKey => tr(locale, "tool.credential_private_key"),
        CredentialKind::Agent => tr(locale, "tool.credential_agent"),
        CredentialKind::Certificate => tr(locale, "tool.credential_certificate"),
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

fn key_algorithm_label(algorithm: &KeyAlgorithm) -> String {
    match algorithm {
        KeyAlgorithm::Ed25519 => "ed25519".to_owned(),
        KeyAlgorithm::Rsa => "rsa".to_owned(),
        KeyAlgorithm::Ecdsa => "ecdsa".to_owned(),
        KeyAlgorithm::Unknown(name) => name.clone(),
    }
}
