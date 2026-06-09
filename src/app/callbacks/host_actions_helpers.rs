//! 主机动作回调的轻量工具函数。

use uuid::Uuid;

use crate::model::{
    CredentialGroupId, CredentialKind, KeyAlgorithm, QuickHostAuthField, QuickHostAuthKind,
    QuickHostDraftField,
};

pub(super) struct CredentialDropTarget {
    pub(super) group_id: Option<CredentialGroupId>,
    pub(super) kind: Option<CredentialKind>,
}

pub(super) fn copy_text_to_clipboard(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }

    #[cfg(target_os = "windows")]
    {
        use std::io::Write;
        use std::process::{Command, Stdio};

        let Ok(mut child) = Command::new("cmd")
            .args(["/C", "clip"])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        else {
            return false;
        };

        let Some(stdin) = child.stdin.as_mut() else {
            return false;
        };
        if stdin.write_all(text.as_bytes()).is_err() {
            return false;
        }
        drop(child.stdin.take());

        child.wait().is_ok_and(|status| status.success())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = text;
        false
    }
}

pub(super) fn credential_secret_text(
    state: &crate::model::AppState,
    row_id: &str,
) -> Option<String> {
    let name = parse_credential_row_id(row_id)?;
    let credential = state
        .storage
        .credentials
        .iter()
        .find(|credential| credential.name == name)?;
    let secret_ref = credential.secret.as_ref()?;
    let secret = state
        .storage
        .secrets
        .iter()
        .find(|secret| &secret.secret_ref == secret_ref)?;

    // 现在的本地存储版本 0 是明文 payload；未来主密码加密接入后，
    // 只需要在这里按 encryption_version 分支解密，不让 UI 常驻明文。
    if secret.encryption_version != 0 {
        return None;
    }

    secret
        .encrypted_payload
        .as_ref()
        .map(|payload| String::from_utf8_lossy(payload).into_owned())
}

pub(super) fn parse_quick_host_field(field: &str) -> Option<QuickHostDraftField> {
    // 这些 key 来自 Slint 表单绑定，必须是稳定协议值，不允许使用本地化文案。
    match field {
        "Name" => Some(QuickHostDraftField::Name),
        "Address" => Some(QuickHostDraftField::Address),
        "Port" => Some(QuickHostDraftField::Port),
        "Username" => Some(QuickHostDraftField::Username),
        "Tags" => Some(QuickHostDraftField::Tags),
        "IconKey" => Some(QuickHostDraftField::IconKey),
        _ => None,
    }
}

pub(super) fn parse_quick_host_auth_kind(kind: &str) -> Option<QuickHostAuthKind> {
    // 认证方式也使用稳定 key，展示文案由 view_model/i18n 决定。
    match kind {
        "Password" => Some(QuickHostAuthKind::Password),
        "Key" => Some(QuickHostAuthKind::Key),
        "ssh-agent" => Some(QuickHostAuthKind::Agent),
        "Certificate" => Some(QuickHostAuthKind::Certificate),
        _ => None,
    }
}

pub(super) fn parse_quick_host_auth_field(field: &str) -> Option<QuickHostAuthField> {
    // 认证字段细节只在核心草稿里建模，UI 不直接构造 AuthProfile。
    match field {
        "AgentSource" => Some(QuickHostAuthField::AgentSource),
        "AgentCustomPipe" => Some(QuickHostAuthField::AgentCustomPipe),
        "PasswordSecretRef" => Some(QuickHostAuthField::PasswordSecretRef),
        "PrivateKeyRef" => Some(QuickHostAuthField::PrivateKeyRef),
        "PassphraseRef" => Some(QuickHostAuthField::PassphraseRef),
        "KeyHint" => Some(QuickHostAuthField::KeyHint),
        "CertificateRef" => Some(QuickHostAuthField::CertificateRef),
        _ => None,
    }
}

pub(super) fn parse_credential_kind(kind: &str) -> Option<CredentialKind> {
    match kind {
        "Password" => Some(CredentialKind::Password),
        "PrivateKey" => Some(CredentialKind::PrivateKey),
        "Agent" => Some(CredentialKind::Agent),
        "Certificate" => Some(CredentialKind::Certificate),
        _ => None,
    }
}

pub(super) fn parse_credential_group_row_id(row_id: &str) -> Option<CredentialGroupId> {
    // 密钥树行使用带前缀的 UI ID，避免 Slint 直接依赖核心 Uuid 类型。
    row_id
        .strip_prefix("credential-group:")
        .and_then(|id| Uuid::parse_str(id).ok())
        .map(CredentialGroupId)
}

pub(super) fn parse_optional_credential_group_row_id(row_id: &str) -> Option<CredentialGroupId> {
    if row_id.is_empty() {
        None
    } else {
        parse_credential_group_row_id(row_id)
    }
}

pub(super) fn parse_credential_drop_target(
    state: &crate::model::AppState,
    row_id: &str,
) -> Option<CredentialDropTarget> {
    if let Some(kind) = row_id
        .strip_prefix("group:")
        .and_then(parse_credential_kind)
    {
        return Some(CredentialDropTarget {
            group_id: None,
            kind: Some(kind),
        });
    }

    let group_id = parse_credential_group_row_id(row_id)?;
    let kind = credential_group_kind_by_id(state, group_id)?;
    Some(CredentialDropTarget {
        group_id: Some(group_id),
        kind: Some(kind),
    })
}

pub(super) fn credential_kind_by_name(
    state: &crate::model::AppState,
    name: &str,
) -> Option<CredentialKind> {
    state
        .storage
        .credentials
        .iter()
        .find(|credential| credential.name == name)
        .map(|credential| credential.kind.clone())
}

pub(super) fn credential_group_kind_by_id(
    state: &crate::model::AppState,
    group_id: CredentialGroupId,
) -> Option<CredentialKind> {
    state
        .storage
        .credential_groups
        .iter()
        .find(|group| group.id == group_id)
        .map(|group| group.kind.clone())
}

pub(super) fn parse_credential_row_id(row_id: &str) -> Option<&str> {
    row_id.strip_prefix("credential:")
}

pub(super) fn parse_key_algorithm(algorithm: &str) -> Option<KeyAlgorithm> {
    match algorithm {
        "Ed25519" => Some(KeyAlgorithm::Ed25519),
        "Rsa" => Some(KeyAlgorithm::Rsa),
        "Ecdsa" => Some(KeyAlgorithm::Ecdsa),
        "Unknown" | "" => None,
        other => Some(KeyAlgorithm::Unknown(other.to_owned())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        AppState, CredentialGroup, CredentialGroupId, CredentialKind, CredentialMetadata,
        KeyAlgorithm, QuickHostAuthField, QuickHostAuthKind, QuickHostDraftField,
        SecretMaterialKind, SecretRecord, SecretRef,
    };

    #[test]
    fn copy_text_to_clipboard_rejects_empty_text() {
        assert!(!copy_text_to_clipboard(""));
    }

    #[test]
    fn credential_secret_text_reads_local_payload_on_demand() {
        let mut state = AppState::default();
        state.storage.upsert_secret(SecretRecord::local_plaintext(
            SecretRef("key:deploy".to_owned()),
            SecretMaterialKind::PrivateKey,
            b"private-key".to_vec(),
        ));
        state.storage.upsert_credential(CredentialMetadata {
            id: crate::model::CredentialId(uuid::Uuid::new_v4()),
            name: "deploy".to_owned(),
            kind: CredentialKind::PrivateKey,
            group_id: None,
            username: Some("ubuntu".to_owned()),
            secret: Some(SecretRef("key:deploy".to_owned())),
            key_algorithm: Some(KeyAlgorithm::Ed25519),
            fingerprint: None,
        });

        assert_eq!(
            credential_secret_text(&state, "credential:deploy").as_deref(),
            Some("private-key")
        );
    }

    #[test]
    fn credential_secret_text_ignores_groups_and_missing_payloads() {
        let mut state = AppState::default();
        state.storage.upsert_credential(CredentialMetadata {
            id: crate::model::CredentialId(uuid::Uuid::new_v4()),
            name: "deploy".to_owned(),
            kind: CredentialKind::PrivateKey,
            group_id: None,
            username: None,
            secret: Some(SecretRef("key:missing".to_owned())),
            key_algorithm: None,
            fingerprint: None,
        });

        assert_eq!(credential_secret_text(&state, "group:PrivateKey"), None);
        assert_eq!(credential_secret_text(&state, "credential:deploy"), None);
    }

    #[test]
    fn quick_host_parsers_accept_stable_ui_keys() {
        assert_eq!(
            parse_quick_host_field("Address"),
            Some(QuickHostDraftField::Address)
        );
        assert_eq!(parse_quick_host_field("地址"), None);
        assert_eq!(
            parse_quick_host_auth_kind("ssh-agent"),
            Some(QuickHostAuthKind::Agent)
        );
        assert_eq!(
            parse_quick_host_auth_field("CertificateRef"),
            Some(QuickHostAuthField::CertificateRef)
        );
    }

    #[test]
    fn credential_parsers_accept_stable_ui_keys() {
        let group_id = CredentialGroupId(uuid::Uuid::new_v4());

        assert_eq!(
            parse_credential_kind("PrivateKey"),
            Some(CredentialKind::PrivateKey)
        );
        assert_eq!(
            parse_credential_group_row_id(&format!("credential-group:{}", group_id.0)),
            Some(group_id)
        );
        assert_eq!(parse_optional_credential_group_row_id(""), None);
        assert_eq!(
            parse_optional_credential_group_row_id(&format!("credential-group:{}", group_id.0)),
            Some(group_id)
        );
        assert_eq!(parse_credential_row_id("credential:deploy"), Some("deploy"));
    }

    #[test]
    fn key_algorithm_parser_handles_known_and_unknown_values() {
        assert_eq!(parse_key_algorithm("Ed25519"), Some(KeyAlgorithm::Ed25519));
        assert_eq!(parse_key_algorithm(""), None);
        assert_eq!(
            parse_key_algorithm("ssh-custom"),
            Some(KeyAlgorithm::Unknown("ssh-custom".to_owned()))
        );
    }

    #[test]
    fn credential_drop_target_resolves_kind_and_group() {
        let mut state = AppState::default();
        let group_id = CredentialGroupId(uuid::Uuid::new_v4());
        state.storage.upsert_credential_group(CredentialGroup {
            id: group_id,
            name: "证书".to_owned(),
            kind: CredentialKind::Certificate,
            parent_id: None,
            sort_order: 0,
        });

        let kind_target = parse_credential_drop_target(&state, "group:PrivateKey")
            .expect("kind drop target should parse");
        assert_eq!(kind_target.group_id, None);
        assert_eq!(kind_target.kind, Some(CredentialKind::PrivateKey));

        let group_target =
            parse_credential_drop_target(&state, &format!("credential-group:{}", group_id.0))
                .expect("group drop target should parse");
        assert_eq!(group_target.group_id, Some(group_id));
        assert_eq!(group_target.kind, Some(CredentialKind::Certificate));
    }
}
