//! 快速新增主机表单和认证草稿。

use std::fmt;

use crate::model::{Host, HostId};

#[path = "quick_host/auth.rs"]
mod auth;
#[path = "quick_host/ui.rs"]
mod ui;

/// 快速新增主机表单草稿。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickHostDraft {
    pub name: String,
    pub address: String,
    pub port: String,
    pub username: String,
    pub tags: String,
    pub auth: QuickHostAuthDraft,
}

impl Default for QuickHostDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            address: String::new(),
            port: "22".to_owned(),
            username: String::new(),
            tags: String::new(),
            auth: QuickHostAuthDraft::default(),
        }
    }
}

impl QuickHostDraft {
    /// 将表单草稿转换为可保存主机。
    pub fn build_host(&self, id: HostId) -> Result<Host, QuickHostDraftError> {
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

        let name = self.name.trim();
        let auth = self.auth.build_auth(username)?;

        Ok(Host {
            id,
            name: if name.is_empty() {
                address.to_owned()
            } else {
                name.to_owned()
            },
            group_id: None,
            tags: parse_tags(&self.tags),
            address: address.to_owned(),
            port,
            auth,
            proxy: None,
            jumps: Vec::new(),
            theme_override: None,
            background_override: None,
        })
    }

    /// 兼容旧调用的快捷入口，内部仍走统一主机构建逻辑。
    pub fn build_agent_host(&self, id: HostId) -> Result<Host, QuickHostDraftError> {
        self.build_host(id)
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
}

/// 快速新增主机认证字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuickHostAuthField {
    PasswordSecretRef,
    PrivateKeyRef,
    PassphraseRef,
    KeyHint,
    CertificateRef,
}

/// 快速新增主机表单校验错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuickHostDraftError {
    EmptyAddress,
    EmptyUsername,
    InvalidPort,
    MissingPasswordSecretRef,
    MissingPrivateKeyRef,
    MissingCertificateRef,
}

impl fmt::Display for QuickHostDraftError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyAddress => f.write_str("地址不能为空"),
            Self::EmptyUsername => f.write_str("用户名不能为空"),
            Self::InvalidPort => f.write_str("端口必须是 1 到 65535"),
            Self::MissingPasswordSecretRef => f.write_str("密码引用不能为空"),
            Self::MissingPrivateKeyRef => f.write_str("私钥引用不能为空"),
            Self::MissingCertificateRef => f.write_str("证书引用不能为空"),
        }
    }
}

fn parse_tags(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
