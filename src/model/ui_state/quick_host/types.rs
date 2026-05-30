//! 快速新增主机草稿类型。

use super::{QuickHostDraftError, auth};

/// 快速新增主机可选的 ssh-agent 来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickHostAgentSource {
    Auto,
    OpenSsh,
    Pageant,
    CustomNamedPipe,
}

impl QuickHostAgentSource {
    /// 返回 UI 与回调使用的稳定标签。
    pub fn label(self) -> &'static str {
        auth::quick_host_agent_source_label(self)
    }
}

/// 快速新增主机的认证方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickHostAuthKind {
    Password,
    Key,
    Agent,
    Certificate,
}

impl QuickHostAuthKind {
    /// 返回认证方式显示名。
    pub fn label(self) -> &'static str {
        auth::quick_host_auth_kind_label(self)
    }
}

/// 快速新增主机的认证字段草稿。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickHostAuthDraft {
    pub kind: QuickHostAuthKind,
    pub agent_source: QuickHostAgentSource,
    pub agent_custom_pipe: String,
    pub password_secret_ref: String,
    pub private_key_ref: String,
    pub passphrase_ref: String,
    pub key_hint: String,
    pub certificate_ref: String,
}

impl Default for QuickHostAuthDraft {
    fn default() -> Self {
        Self {
            kind: QuickHostAuthKind::Agent,
            agent_source: QuickHostAgentSource::Auto,
            agent_custom_pipe: String::new(),
            password_secret_ref: String::new(),
            private_key_ref: String::new(),
            passphrase_ref: String::new(),
            key_hint: String::new(),
            certificate_ref: String::new(),
        }
    }
}

impl QuickHostAuthDraft {
    /// 将认证草稿转换为主机认证配置。
    pub(super) fn build_auth(
        &self,
        username: &str,
    ) -> Result<crate::model::AuthProfile, QuickHostDraftError> {
        auth::build_quick_host_auth(self, username)
    }
}

/// 快速新增主机表单字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickHostDraftField {
    Name,
    Address,
    Port,
    Username,
    Tags,
    IconKey,
}

/// 快速新增主机认证字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickHostAuthField {
    AgentSource,
    AgentCustomPipe,
    PasswordSecretRef,
    PrivateKeyRef,
    PassphraseRef,
    KeyHint,
    CertificateRef,
}
