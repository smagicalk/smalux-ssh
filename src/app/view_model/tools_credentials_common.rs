//! 凭据工具页的通用展示辅助函数。

use crate::model::{
    CredentialGroup, CredentialGroupId, CredentialKind, CredentialMetadata, KeyAlgorithm,
    SecretRecord,
};

use super::i18n::{Locale, tr};

pub(super) fn credential_visible_in_security(kind: &CredentialKind) -> bool {
    !matches!(kind, CredentialKind::Agent)
}

pub(super) fn credential_row_id(credential: &CredentialMetadata) -> String {
    format!("credential:{}", credential.name)
}

pub(super) fn credential_storage_label(
    credential: &CredentialMetadata,
    secret_available: bool,
    locale: Locale,
    empty: &str,
) -> String {
    if credential.secret.is_none() {
        return empty.to_owned();
    }
    if secret_available {
        tr(locale, "security.storage_local").to_owned()
    } else {
        tr(locale, "security.storage_missing").to_owned()
    }
}

pub(super) fn credential_group_path(
    groups: &[CredentialGroup],
    group_id: Option<CredentialGroupId>,
    kind: &CredentialKind,
    locale: Locale,
) -> String {
    let Some(group_id) = group_id else {
        return credential_group_label(kind, locale).to_owned();
    };

    let Some(group) = groups.iter().find(|group| group.id == group_id) else {
        return credential_group_label(kind, locale).to_owned();
    };

    let mut names = vec![group.name.clone()];
    let mut parent_id = group.parent_id;
    while let Some(id) = parent_id {
        let Some(parent) = groups.iter().find(|group| group.id == id) else {
            break;
        };
        names.push(parent.name.clone());
        parent_id = parent.parent_id;
    }
    names.reverse();
    names.join(" / ")
}

pub(super) fn credential_secret_available(
    credential: &CredentialMetadata,
    secrets: &[SecretRecord],
) -> bool {
    let Some(secret_ref) = credential.secret.as_ref() else {
        return false;
    };
    secrets.iter().any(|secret| {
        &secret.secret_ref == secret_ref
            && secret.encryption_version == 0
            && secret.encrypted_payload.is_some()
    })
}

pub(super) fn credential_count_label(count: usize, locale: Locale) -> String {
    format!("{} {}", count, tr(locale, "security.count_suffix"))
}

pub(super) fn credential_matches(
    credential: &CredentialMetadata,
    query: &str,
    locale: Locale,
) -> bool {
    query.is_empty()
        || credential.name.to_lowercase().contains(query)
        || credential_kind_label(&credential.kind, locale)
            .to_lowercase()
            .contains(query)
        || credential_group_label(&credential.kind, locale)
            .to_lowercase()
            .contains(query)
        || credential
            .username
            .as_ref()
            .is_some_and(|username| username.to_lowercase().contains(query))
        || credential
            .secret
            .as_ref()
            .is_some_and(|secret| secret.0.to_lowercase().contains(query))
        || credential
            .key_algorithm
            .as_ref()
            .map(key_algorithm_label)
            .is_some_and(|algorithm| algorithm.to_lowercase().contains(query))
        || credential
            .fingerprint
            .as_ref()
            .is_some_and(|fingerprint| fingerprint.to_lowercase().contains(query))
}

pub(super) fn credential_group_label(kind: &CredentialKind, locale: Locale) -> &'static str {
    match kind {
        CredentialKind::Password => tr(locale, "security.passwords"),
        CredentialKind::PrivateKey => tr(locale, "security.private_keys"),
        CredentialKind::Agent => tr(locale, "security.agents"),
        CredentialKind::Certificate => tr(locale, "security.certificates"),
    }
}

pub(super) fn credential_kind_key(kind: &CredentialKind) -> &'static str {
    match kind {
        CredentialKind::Password => "Password",
        CredentialKind::PrivateKey => "PrivateKey",
        CredentialKind::Agent => "Agent",
        CredentialKind::Certificate => "Certificate",
    }
}

pub(super) fn credential_icon_key(kind: &CredentialKind) -> &'static str {
    match kind {
        CredentialKind::Password => "shield",
        CredentialKind::PrivateKey => "key",
        CredentialKind::Agent => "terminal",
        CredentialKind::Certificate => "database",
    }
}

pub(super) fn credential_kind_label(kind: &CredentialKind, locale: Locale) -> &'static str {
    match kind {
        CredentialKind::Password => tr(locale, "tool.credential_password"),
        CredentialKind::PrivateKey => tr(locale, "tool.credential_private_key"),
        CredentialKind::Agent => tr(locale, "tool.credential_agent"),
        CredentialKind::Certificate => tr(locale, "tool.credential_certificate"),
    }
}

pub(super) fn key_algorithm_label(algorithm: &KeyAlgorithm) -> String {
    match algorithm {
        KeyAlgorithm::Ed25519 => "ed25519".to_owned(),
        KeyAlgorithm::Rsa => "rsa".to_owned(),
        KeyAlgorithm::Ecdsa => "ecdsa".to_owned(),
        KeyAlgorithm::Unknown(name) => name.clone(),
    }
}

pub(super) fn key_algorithm_key(algorithm: &KeyAlgorithm) -> String {
    match algorithm {
        KeyAlgorithm::Ed25519 => "Ed25519".to_owned(),
        KeyAlgorithm::Rsa => "Rsa".to_owned(),
        KeyAlgorithm::Ecdsa => "Ecdsa".to_owned(),
        KeyAlgorithm::Unknown(name) => name.clone(),
    }
}
