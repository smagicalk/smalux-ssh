//! 快速新增主机表单和认证草稿。

use std::fmt;

use crate::model::{AuthProfile, Host, HostId, SecretRef};

use super::UiState;

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
        match self {
            Self::Password => "Password",
            Self::Key => "Key",
            Self::Agent => "ssh-agent",
            Self::Certificate => "Certificate",
        }
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
    fn build_auth(&self, username: &str) -> Result<AuthProfile, QuickHostDraftError> {
        match self.kind {
            QuickHostAuthKind::Password => {
                let secret_ref = self.password_secret_ref.trim();
                if secret_ref.is_empty() {
                    return Err(QuickHostDraftError::MissingPasswordSecretRef);
                }

                Ok(AuthProfile::Password {
                    username: username.to_owned(),
                    secret: SecretRef(secret_ref.to_owned()),
                })
            }
            QuickHostAuthKind::Key => {
                let private_key_ref = self.private_key_ref.trim();
                if private_key_ref.is_empty() {
                    return Err(QuickHostDraftError::MissingPrivateKeyRef);
                }

                let passphrase_ref = self.passphrase_ref.trim();

                Ok(AuthProfile::Key {
                    username: username.to_owned(),
                    key: SecretRef(private_key_ref.to_owned()),
                    passphrase: if passphrase_ref.is_empty() {
                        None
                    } else {
                        Some(SecretRef(passphrase_ref.to_owned()))
                    },
                })
            }
            QuickHostAuthKind::Agent => {
                let key_hint = self.key_hint.trim();

                Ok(AuthProfile::Agent {
                    username: username.to_owned(),
                    key_hint: if key_hint.is_empty() {
                        None
                    } else {
                        Some(key_hint.to_owned())
                    },
                })
            }
            QuickHostAuthKind::Certificate => {
                let private_key_ref = self.private_key_ref.trim();
                if private_key_ref.is_empty() {
                    return Err(QuickHostDraftError::MissingPrivateKeyRef);
                }

                let certificate_ref = self.certificate_ref.trim();
                if certificate_ref.is_empty() {
                    return Err(QuickHostDraftError::MissingCertificateRef);
                }

                Ok(AuthProfile::Certificate {
                    username: username.to_owned(),
                    key: SecretRef(private_key_ref.to_owned()),
                    passphrase: if self.passphrase_ref.trim().is_empty() {
                        None
                    } else {
                        Some(SecretRef(self.passphrase_ref.trim().to_owned()))
                    },
                    certificate: SecretRef(certificate_ref.to_owned()),
                })
            }
        }
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

impl UiState {
    /// 更新快速新增主机表单字段。
    pub fn set_quick_host_field(&mut self, field: QuickHostDraftField, value: impl Into<String>) {
        let value = value.into();

        match field {
            QuickHostDraftField::Name => self.quick_host.name = value,
            QuickHostDraftField::Address => self.quick_host.address = value,
            QuickHostDraftField::Port => self.quick_host.port = value,
            QuickHostDraftField::Username => self.quick_host.username = value,
            QuickHostDraftField::Tags => self.quick_host.tags = value,
        }
    }

    /// 更新快速新增主机的认证方式。
    pub fn set_quick_host_auth_kind(&mut self, kind: QuickHostAuthKind) {
        self.quick_host.auth.kind = kind;
    }

    /// 更新快速新增主机的认证字段。
    pub fn set_quick_host_auth_field(
        &mut self,
        field: QuickHostAuthField,
        value: impl Into<String>,
    ) {
        let value = value.into();

        match field {
            QuickHostAuthField::PasswordSecretRef => {
                self.quick_host.auth.password_secret_ref = value
            }
            QuickHostAuthField::PrivateKeyRef => self.quick_host.auth.private_key_ref = value,
            QuickHostAuthField::PassphraseRef => self.quick_host.auth.passphrase_ref = value,
            QuickHostAuthField::KeyHint => self.quick_host.auth.key_hint = value,
            QuickHostAuthField::CertificateRef => self.quick_host.auth.certificate_ref = value,
        }
    }

    /// 清空快速新增主机表单，保留默认 SSH 端口。
    pub fn reset_quick_host(&mut self) {
        self.quick_host = QuickHostDraft::default();
    }
}

fn parse_tags(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
