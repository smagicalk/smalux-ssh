//! 主机草稿与领域主机之间的转换。
//!
//! 这个模块只处理“值到值”的转换，不依赖具体 UI 框架，也不依赖 `UiState` 的
//! 其他运行态字段。桌面层、CLI 或未来其他 UI 都可以复用它。

use crate::model::{
    AgentSource, AuthProfile, Host, HostId, QuickHostAgentSource, QuickHostAuthDraft,
    QuickHostAuthKind, QuickHostDraft, QuickHostDraftError,
};

pub const DEFAULT_QUICK_HOST_ICON_KEY: &str = "server";
pub const MAX_QUICK_HOST_NAME_CHARS: usize = 48;

/// 从已保存主机生成编辑草稿。
pub fn quick_host_draft_from_host(host: &Host) -> QuickHostDraft {
    let (username, auth) = auth_draft_from_profile(&host.auth);

    QuickHostDraft {
        editing_host_id: Some(host.id),
        group_id: host.group_id,
        network: host.network.clone(),
        name: host.name.clone(),
        address: host.address.clone(),
        port: host.port.to_string(),
        username,
        icon_key: normalized_icon_key(&host.icon_key),
        tags: host.tags.join(", "),
        auth,
    }
}

/// 将主机草稿转换为可保存主机。
pub fn build_host_from_draft(
    draft: &QuickHostDraft,
    id: HostId,
    existing: Option<&Host>,
) -> Result<Host, QuickHostDraftError> {
    let address = draft.address.trim();
    if address.is_empty() {
        return Err(QuickHostDraftError::EmptyAddress);
    }

    let username = draft.username.trim();
    if username.is_empty() {
        return Err(QuickHostDraftError::EmptyUsername);
    }

    let port = draft
        .port
        .trim()
        .parse::<u16>()
        .map_err(|_| QuickHostDraftError::InvalidPort)?;
    if port == 0 {
        return Err(QuickHostDraftError::InvalidPort);
    }

    let name = truncate_host_name(draft.name.trim());
    let auth = build_quick_host_auth(&draft.auth, username)?;

    let mut host = Host {
        id,
        name: if name.is_empty() {
            address.to_owned()
        } else {
            name
        },
        group_id: draft.group_id,
        icon_key: normalized_icon_key(&draft.icon_key),
        tags: parse_tags(&draft.tags),
        address: address.to_owned(),
        port,
        auth,
        network: draft.network.clone(),
        proxies: Vec::new(),
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    };

    if let Some(existing) = existing {
        host.proxies = existing.proxies.clone();
        host.jumps = existing.jumps.clone();
        host.theme_override = existing.theme_override.clone();
        host.background_override = existing.background_override.clone();
    }

    Ok(host)
}

pub fn truncate_host_name(name: &str) -> String {
    name.chars().take(MAX_QUICK_HOST_NAME_CHARS).collect()
}

pub fn normalized_icon_key(icon_key: &str) -> String {
    let icon_key = icon_key.trim();
    if icon_key.is_empty() {
        DEFAULT_QUICK_HOST_ICON_KEY.to_owned()
    } else {
        icon_key.to_owned()
    }
}

pub fn build_quick_host_auth(
    draft: &QuickHostAuthDraft,
    username: &str,
) -> Result<AuthProfile, QuickHostDraftError> {
    match draft.kind {
        QuickHostAuthKind::Password => password_auth(draft, username),
        QuickHostAuthKind::Key => key_auth(draft, username),
        QuickHostAuthKind::Agent => agent_auth(draft, username),
        QuickHostAuthKind::Certificate => certificate_auth(draft, username),
    }
}

fn password_auth(
    draft: &QuickHostAuthDraft,
    username: &str,
) -> Result<AuthProfile, QuickHostDraftError> {
    let secret_ref = draft.password_secret_ref.trim();
    if secret_ref.is_empty() {
        return Err(QuickHostDraftError::MissingPasswordSecretRef);
    }

    Ok(AuthProfile::Password {
        username: username.to_owned(),
        secret: crate::model::SecretRef(secret_ref.to_owned()),
    })
}

fn key_auth(
    draft: &QuickHostAuthDraft,
    username: &str,
) -> Result<AuthProfile, QuickHostDraftError> {
    let private_key_ref = draft.private_key_ref.trim();
    if private_key_ref.is_empty() {
        return Err(QuickHostDraftError::MissingPrivateKeyRef);
    }

    let passphrase_ref = draft.passphrase_ref.trim();

    Ok(AuthProfile::Key {
        username: username.to_owned(),
        key: crate::model::SecretRef(private_key_ref.to_owned()),
        passphrase: if passphrase_ref.is_empty() {
            None
        } else {
            Some(crate::model::SecretRef(passphrase_ref.to_owned()))
        },
    })
}

fn agent_auth(
    draft: &QuickHostAuthDraft,
    username: &str,
) -> Result<AuthProfile, QuickHostDraftError> {
    let key_hint = draft.key_hint.trim();
    let source = agent_source(draft)?;

    Ok(AuthProfile::Agent {
        username: username.to_owned(),
        source,
        key_hint: if key_hint.is_empty() {
            None
        } else {
            Some(key_hint.to_owned())
        },
    })
}

fn agent_source(draft: &QuickHostAuthDraft) -> Result<AgentSource, QuickHostDraftError> {
    match draft.agent_source {
        QuickHostAgentSource::Auto => Ok(AgentSource::Auto),
        QuickHostAgentSource::OpenSsh => Ok(AgentSource::OpenSsh),
        QuickHostAgentSource::Pageant => Ok(AgentSource::Pageant),
        QuickHostAgentSource::CustomNamedPipe => {
            let pipe = draft.agent_custom_pipe.trim();
            if pipe.is_empty() {
                return Err(QuickHostDraftError::MissingAgentPipePath);
            }
            Ok(AgentSource::CustomNamedPipe(pipe.to_owned()))
        }
    }
}

fn certificate_auth(
    draft: &QuickHostAuthDraft,
    username: &str,
) -> Result<AuthProfile, QuickHostDraftError> {
    let private_key_ref = draft.private_key_ref.trim();
    if private_key_ref.is_empty() {
        return Err(QuickHostDraftError::MissingPrivateKeyRef);
    }

    let certificate_ref = draft.certificate_ref.trim();
    if certificate_ref.is_empty() {
        return Err(QuickHostDraftError::MissingCertificateRef);
    }

    Ok(AuthProfile::Certificate {
        username: username.to_owned(),
        key: crate::model::SecretRef(private_key_ref.to_owned()),
        passphrase: if draft.passphrase_ref.trim().is_empty() {
            None
        } else {
            Some(crate::model::SecretRef(
                draft.passphrase_ref.trim().to_owned(),
            ))
        },
        certificate: crate::model::SecretRef(certificate_ref.to_owned()),
    })
}

fn auth_draft_from_profile(auth: &AuthProfile) -> (String, QuickHostAuthDraft) {
    match auth {
        AuthProfile::Password { username, secret } => (
            username.clone(),
            QuickHostAuthDraft {
                kind: QuickHostAuthKind::Password,
                password_secret_ref: secret.0.clone(),
                ..QuickHostAuthDraft::default()
            },
        ),
        AuthProfile::Key {
            username,
            key,
            passphrase,
        } => (
            username.clone(),
            QuickHostAuthDraft {
                kind: QuickHostAuthKind::Key,
                private_key_ref: key.0.clone(),
                passphrase_ref: passphrase
                    .as_ref()
                    .map(|secret| secret.0.clone())
                    .unwrap_or_default(),
                ..QuickHostAuthDraft::default()
            },
        ),
        AuthProfile::Agent {
            username,
            source,
            key_hint,
        } => (
            username.clone(),
            QuickHostAuthDraft {
                kind: QuickHostAuthKind::Agent,
                agent_source: quick_host_agent_source(source),
                agent_custom_pipe: custom_agent_pipe(source),
                key_hint: key_hint.clone().unwrap_or_default(),
                ..QuickHostAuthDraft::default()
            },
        ),
        AuthProfile::Certificate {
            username,
            key,
            passphrase,
            certificate,
        } => (
            username.clone(),
            QuickHostAuthDraft {
                kind: QuickHostAuthKind::Certificate,
                private_key_ref: key.0.clone(),
                passphrase_ref: passphrase
                    .as_ref()
                    .map(|secret| secret.0.clone())
                    .unwrap_or_default(),
                certificate_ref: certificate.0.clone(),
                ..QuickHostAuthDraft::default()
            },
        ),
    }
}

fn quick_host_agent_source(source: &AgentSource) -> QuickHostAgentSource {
    match source {
        AgentSource::Auto => QuickHostAgentSource::Auto,
        AgentSource::OpenSsh => QuickHostAgentSource::OpenSsh,
        AgentSource::Pageant => QuickHostAgentSource::Pageant,
        AgentSource::CustomNamedPipe(_) => QuickHostAgentSource::CustomNamedPipe,
    }
}

fn custom_agent_pipe(source: &AgentSource) -> String {
    match source {
        AgentSource::CustomNamedPipe(pipe) => pipe.clone(),
        _ => String::new(),
    }
}

fn parse_tags(raw: &str) -> Vec<String> {
    raw.split([',', '，', '、', ';', '；'])
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
