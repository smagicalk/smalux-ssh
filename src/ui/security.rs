//! 凭据和 Known Hosts 管理视图。
//!
//! 这一层只负责展示和触发删除/信任操作，不直接碰底层存储实现。

use iced::{
    Element, Length,
    widget::{button, column, row, text},
};

use crate::model::{AppState, CredentialKind, KeyAlgorithm, Message};

/// 渲染安全资产管理区域。
pub fn view(state: &AppState) -> Element<'_, Message> {
    column![
        text("Security").size(22),
        credentials_panel(state),
        known_hosts_panel(state),
    ]
    .spacing(12)
    .into()
}

fn credentials_panel(state: &AppState) -> Element<'_, Message> {
    let mut panel = column![text("Credentials").size(18)].spacing(8);

    if state.storage.credentials.is_empty() {
        return panel.push(text("No credentials saved.")).into();
    }

    for credential in &state.storage.credentials {
        panel = panel.push(
            column![
                row![
                    text(credential_summary(credential)).width(Length::Fill),
                    button("Remove").on_press(Message::RemoveCredential {
                        name: credential.name.clone(),
                    }),
                ]
                .spacing(8),
                text(credential_details(credential)),
            ]
            .spacing(4),
        );
    }

    panel.into()
}

fn known_hosts_panel(state: &AppState) -> Element<'_, Message> {
    let mut panel = column![text("Known Hosts").size(18)].spacing(8);

    if state.storage.known_hosts.is_empty() {
        return panel.push(text("No known hosts saved.")).into();
    }

    for entry in &state.storage.known_hosts {
        let actions = if entry.trusted {
            row![
                text("Trusted"),
                button("Remove").on_press(Message::RemoveKnownHost {
                    host: entry.host.clone(),
                    port: entry.port,
                }),
            ]
        } else {
            row![
                button("Trust").on_press(Message::TrustKnownHost {
                    host: entry.host.clone(),
                    port: entry.port,
                }),
                button("Remove").on_press(Message::RemoveKnownHost {
                    host: entry.host.clone(),
                    port: entry.port,
                }),
            ]
        };

        panel = panel.push(
            column![
                row![
                    text(known_host_summary(entry)).width(Length::Fill),
                    actions.spacing(8),
                ]
                .spacing(8),
                text(known_host_details(entry)),
            ]
            .spacing(4),
        );
    }

    panel.into()
}

fn credential_summary(credential: &crate::model::CredentialMetadata) -> String {
    format!(
        "{} | {} | user: {} | secret ref: {}",
        credential.name,
        credential_kind_label(&credential.kind),
        credential.username.as_deref().unwrap_or("-"),
        credential
            .secret
            .as_ref()
            .map(|secret| secret.0.as_str())
            .unwrap_or("none"),
    )
}

fn credential_details(credential: &crate::model::CredentialMetadata) -> String {
    let algorithm = credential
        .key_algorithm
        .as_ref()
        .map(key_algorithm_label)
        .unwrap_or_else(|| "n/a".to_owned());
    let fingerprint = credential
        .fingerprint
        .as_deref()
        .unwrap_or("n/a")
        .to_owned();

    format!("algorithm: {} | fingerprint: {}", algorithm, fingerprint)
}

fn known_host_summary(entry: &crate::model::KnownHostEntry) -> String {
    format!(
        "{}:{} | {} | {}",
        entry.host,
        entry.port,
        key_algorithm_label(&entry.key_algorithm),
        trusted_label(entry.trusted),
    )
}

fn known_host_details(entry: &crate::model::KnownHostEntry) -> String {
    format!("fingerprint: {}", entry.fingerprint)
}

fn credential_kind_label(kind: &CredentialKind) -> &'static str {
    match kind {
        CredentialKind::Password => "password",
        CredentialKind::PrivateKey => "private key",
        CredentialKind::Agent => "ssh-agent",
        CredentialKind::Certificate => "certificate",
    }
}

fn key_algorithm_label(algorithm: &KeyAlgorithm) -> String {
    match algorithm {
        KeyAlgorithm::Ed25519 => "ed25519".to_owned(),
        KeyAlgorithm::Rsa => "rsa".to_owned(),
        KeyAlgorithm::Ecdsa => "ecdsa".to_owned(),
        KeyAlgorithm::Unknown(value) => format!("unknown({value})"),
    }
}

fn trusted_label(trusted: bool) -> &'static str {
    if trusted { "trusted" } else { "untrusted" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        CredentialKind, CredentialMetadata, KeyAlgorithm, KnownHostEntry, SecretRef,
    };

    #[test]
    fn security_view_accepts_empty_state() {
        let state = AppState::default();

        let _element = view(&state);
    }

    #[test]
    fn security_view_accepts_populated_state() {
        let mut state = AppState::default();
        state.storage.upsert_credential(CredentialMetadata {
            name: "deploy-password".to_owned(),
            kind: CredentialKind::Password,
            username: Some("deploy".to_owned()),
            secret: Some(SecretRef("password:deploy".to_owned())),
            key_algorithm: None,
            fingerprint: None,
        });
        state.storage.upsert_known_host(KnownHostEntry {
            host: "example.com".to_owned(),
            port: 22,
            key_algorithm: KeyAlgorithm::Ed25519,
            fingerprint: "SHA256:demo".to_owned(),
            trusted: false,
        });

        let _element = view(&state);
    }
}
