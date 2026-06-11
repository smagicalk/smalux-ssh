//! 快速新增主机表单草稿构建。

use crate::model::{AgentSource, AuthProfile, GroupId, Host, HostId, HostNetworkSelection};

use super::{QuickHostAgentSource, QuickHostAuthDraft, QuickHostAuthKind, QuickHostDraftError};

pub const DEFAULT_QUICK_HOST_ICON_KEY: &str = "server";
pub const MAX_QUICK_HOST_NAME_CHARS: usize = 48;

/// 快速新增主机表单草稿。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickHostDraft {
    pub editing_host_id: Option<HostId>,
    pub group_id: Option<GroupId>,
    pub network: HostNetworkSelection,
    pub name: String,
    pub address: String,
    pub port: String,
    pub username: String,
    pub icon_key: String,
    pub tags: String,
    pub auth: QuickHostAuthDraft,
}

impl Default for QuickHostDraft {
    fn default() -> Self {
        Self {
            editing_host_id: None,
            group_id: None,
            network: HostNetworkSelection::default(),
            name: String::new(),
            address: String::new(),
            port: "22".to_owned(),
            username: String::new(),
            icon_key: DEFAULT_QUICK_HOST_ICON_KEY.to_owned(),
            tags: String::new(),
            auth: QuickHostAuthDraft::default(),
        }
    }
}

impl QuickHostDraft {
    /// 从已保存主机生成编辑草稿。
    pub fn from_host(host: &Host) -> Self {
        let (username, auth) = auth_draft_from_profile(&host.auth);

        Self {
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

    /// 将表单草稿转换为可保存主机。
    pub fn build_host(&self, id: HostId) -> Result<Host, QuickHostDraftError> {
        self.build_host_with_existing(id, None)
    }

    /// 将表单草稿转换为可保存主机，并保留当前表单暂未暴露的高级配置。
    pub fn build_host_with_existing(
        &self,
        id: HostId,
        existing: Option<&Host>,
    ) -> Result<Host, QuickHostDraftError> {
        let address = self.address.trim();
        if address.is_empty() {
            return Err(QuickHostDraftError::EmptyAddress);
        }

        let username = self.username.trim();
        if username.is_empty() {
            return Err(QuickHostDraftError::EmptyUsername);
        }

        let port = self
            .port
            .trim()
            .parse::<u16>()
            .map_err(|_| QuickHostDraftError::InvalidPort)?;
        if port == 0 {
            return Err(QuickHostDraftError::InvalidPort);
        }

        let name = truncate_host_name(self.name.trim());
        let auth = self.auth.build_auth(username)?;

        let mut host = Host {
            id,
            name: if name.is_empty() {
                address.to_owned()
            } else {
                name
            },
            group_id: self.group_id,
            icon_key: normalized_icon_key(&self.icon_key),
            tags: parse_tags(&self.tags),
            address: address.to_owned(),
            port,
            auth,
            network: self.network.clone(),
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

    /// 兼容旧调用的快捷入口，内部仍走统一主机构建逻辑。
    pub fn build_agent_host(&self, id: HostId) -> Result<Host, QuickHostDraftError> {
        self.build_host(id)
    }
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
