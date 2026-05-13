//! 快速新增主机表单和认证草稿。

use std::fmt;

use crate::model::{AuthProfile, Host, HostId, SecretRef};

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

fn parse_tags(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn host_id() -> HostId {
        HostId(Uuid::new_v4())
    }

    #[test]
    fn quick_host_draft_builds_agent_host() {
        let draft = QuickHostDraft {
            name: "prod".to_owned(),
            address: "prod.example.com".to_owned(),
            port: "2222".to_owned(),
            username: "deploy".to_owned(),
            tags: "prod, linux".to_owned(),
            auth: QuickHostAuthDraft {
                kind: QuickHostAuthKind::Agent,
                key_hint: "id_ed25519".to_owned(),
                ..QuickHostAuthDraft::default()
            },
        };

        let host = draft
            .build_agent_host(host_id())
            .expect("有效主机草稿应该可以生成主机配置");

        assert_eq!(host.name, "prod");
        assert_eq!(host.address, "prod.example.com");
        assert_eq!(host.port, 2222);
        assert_eq!(host.tags, vec!["prod", "linux"]);
        assert!(matches!(
            host.auth,
            AuthProfile::Agent {
                username,
                key_hint: Some(key_hint),
            } if username == "deploy" && key_hint == "id_ed25519"
        ));
    }

    #[test]
    fn quick_host_draft_validates_required_fields() {
        let draft = QuickHostDraft::default();

        assert_eq!(
            draft.build_host(host_id()),
            Err(QuickHostDraftError::EmptyAddress)
        );

        let missing_user = QuickHostDraft {
            address: "example.com".to_owned(),
            ..QuickHostDraft::default()
        };
        assert_eq!(
            missing_user.build_host(host_id()),
            Err(QuickHostDraftError::EmptyUsername)
        );

        let missing_password_ref = QuickHostDraft {
            address: "example.com".to_owned(),
            username: "root".to_owned(),
            auth: QuickHostAuthDraft {
                kind: QuickHostAuthKind::Password,
                ..QuickHostAuthDraft::default()
            },
            ..QuickHostDraft::default()
        };
        assert_eq!(
            missing_password_ref.build_host(host_id()),
            Err(QuickHostDraftError::MissingPasswordSecretRef)
        );

        let missing_private_key_ref = QuickHostDraft {
            address: "example.com".to_owned(),
            username: "deploy".to_owned(),
            auth: QuickHostAuthDraft {
                kind: QuickHostAuthKind::Key,
                ..QuickHostAuthDraft::default()
            },
            ..QuickHostDraft::default()
        };
        assert_eq!(
            missing_private_key_ref.build_host(host_id()),
            Err(QuickHostDraftError::MissingPrivateKeyRef)
        );

        let missing_certificate_ref = QuickHostDraft {
            address: "example.com".to_owned(),
            username: "deploy".to_owned(),
            auth: QuickHostAuthDraft {
                kind: QuickHostAuthKind::Certificate,
                private_key_ref: "key:deploy".to_owned(),
                ..QuickHostAuthDraft::default()
            },
            ..QuickHostDraft::default()
        };
        assert_eq!(
            missing_certificate_ref.build_host(host_id()),
            Err(QuickHostDraftError::MissingCertificateRef)
        );
    }

    #[test]
    fn quick_host_draft_builds_password_host() {
        let draft = QuickHostDraft {
            name: "root".to_owned(),
            address: "root.example.com".to_owned(),
            port: "22".to_owned(),
            username: "root".to_owned(),
            tags: String::new(),
            auth: QuickHostAuthDraft {
                kind: QuickHostAuthKind::Password,
                password_secret_ref: "password:root".to_owned(),
                ..QuickHostAuthDraft::default()
            },
        };

        let host = draft
            .build_host(host_id())
            .expect("密码草稿应该可以生成主机配置");

        assert!(matches!(
            host.auth,
            AuthProfile::Password {
                username,
                secret: SecretRef(ref secret_ref),
            } if username == "root" && secret_ref == "password:root"
        ));
    }

    #[test]
    fn quick_host_draft_builds_key_host_with_passphrase() {
        let draft = QuickHostDraft {
            name: "deploy".to_owned(),
            address: "deploy.example.com".to_owned(),
            port: "2200".to_owned(),
            username: "deploy".to_owned(),
            tags: String::new(),
            auth: QuickHostAuthDraft {
                kind: QuickHostAuthKind::Key,
                private_key_ref: "key:deploy".to_owned(),
                passphrase_ref: "passphrase:deploy".to_owned(),
                ..QuickHostAuthDraft::default()
            },
        };

        let host = draft
            .build_host(host_id())
            .expect("私钥草稿应该可以生成主机配置");

        assert!(matches!(
            host.auth,
            AuthProfile::Key {
                username,
                key: SecretRef(ref key_ref),
                passphrase: Some(SecretRef(ref passphrase_ref)),
            } if username == "deploy"
                && key_ref == "key:deploy"
                && passphrase_ref == "passphrase:deploy"
        ));
    }

    #[test]
    fn quick_host_draft_builds_certificate_host() {
        let draft = QuickHostDraft {
            name: "cert".to_owned(),
            address: "cert.example.com".to_owned(),
            port: "2222".to_owned(),
            username: "cert-user".to_owned(),
            tags: String::new(),
            auth: QuickHostAuthDraft {
                kind: QuickHostAuthKind::Certificate,
                private_key_ref: "key:cert-user".to_owned(),
                certificate_ref: "cert:cert-user".to_owned(),
                ..QuickHostAuthDraft::default()
            },
        };

        let host = draft
            .build_host(host_id())
            .expect("证书草稿应该可以生成主机配置");

        assert!(matches!(
            host.auth,
            AuthProfile::Certificate {
                username,
                key: SecretRef(ref key_ref),
                certificate: SecretRef(ref certificate_ref),
            } if username == "cert-user"
                && key_ref == "key:cert-user"
                && certificate_ref == "cert:cert-user"
        ));
    }
}
