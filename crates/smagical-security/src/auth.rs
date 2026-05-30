//! SSH 后端认证材料描述。
//!
//! 这里是后端执行器能理解的认证引用模型。它从核心 `AuthProfile` 转换而来，但仍然不包含
//! 明文秘密；明文解析只允许发生在 `AuthResolver` 中。

use smagical_core::{AgentSource, AuthProfile, SecretRef};

/// 后端执行器可解析的认证材料引用。
///
/// 密码、私钥和证书仍然只通过 `SecretRef` 间接引用，真实明文读取由安全层负责。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendAuth {
    /// 密码认证引用。
    Password { username: String, secret: SecretRef },
    /// 私钥认证引用。
    Key {
        username: String,
        key: SecretRef,
        passphrase: Option<SecretRef>,
    },
    /// ssh-agent 认证，不需要读取 SecretStore。
    Agent {
        username: String,
        source: AgentSource,
        key_hint: Option<String>,
    },
    /// OpenSSH 证书认证引用。
    Certificate {
        username: String,
        key: SecretRef,
        passphrase: Option<SecretRef>,
        certificate: SecretRef,
    },
}

impl From<&AuthProfile> for BackendAuth {
    fn from(profile: &AuthProfile) -> Self {
        // 只复制引用和用户名，不在转换阶段读取任何秘密。
        match profile {
            AuthProfile::Password { username, secret } => Self::Password {
                username: username.clone(),
                secret: secret.clone(),
            },
            AuthProfile::Key {
                username,
                key,
                passphrase,
            } => Self::Key {
                username: username.clone(),
                key: key.clone(),
                passphrase: passphrase.clone(),
            },
            AuthProfile::Agent {
                username,
                source,
                key_hint,
            } => Self::Agent {
                username: username.clone(),
                source: source.clone(),
                key_hint: key_hint.clone(),
            },
            AuthProfile::Certificate {
                username,
                key,
                passphrase,
                certificate,
            } => Self::Certificate {
                username: username.clone(),
                key: key.clone(),
                passphrase: passphrase.clone(),
                certificate: certificate.clone(),
            },
        }
    }
}

impl BackendAuth {
    /// 返回可显示的用户名，避免调用方匹配所有认证分支。
    pub fn username(&self) -> &str {
        match self {
            Self::Password { username, .. }
            | Self::Key { username, .. }
            | Self::Agent { username, .. }
            | Self::Certificate { username, .. } => username,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_auth_preserves_secret_references() {
        let profile = AuthProfile::Key {
            username: "deploy".to_owned(),
            key: SecretRef("key:deploy".to_owned()),
            passphrase: Some(SecretRef("passphrase:deploy".to_owned())),
        };

        let auth = BackendAuth::from(&profile);

        assert_eq!(auth.username(), "deploy");
        assert!(matches!(
            auth,
            BackendAuth::Key {
                key: SecretRef(ref value),
                ..
            } if value == "key:deploy"
        ));
    }

    #[test]
    fn backend_auth_preserves_password_secret_reference() {
        let profile = AuthProfile::Password {
            username: "root".to_owned(),
            secret: SecretRef("password:root".to_owned()),
        };

        let auth = BackendAuth::from(&profile);

        assert_eq!(auth.username(), "root");
        assert_eq!(
            auth,
            BackendAuth::Password {
                username: "root".to_owned(),
                secret: SecretRef("password:root".to_owned()),
            }
        );
    }

    #[test]
    fn backend_auth_preserves_agent_key_hint() {
        let profile = AuthProfile::Agent {
            username: "agent-user".to_owned(),
            source: AgentSource::Auto,
            key_hint: Some("id_ed25519".to_owned()),
        };

        let auth = BackendAuth::from(&profile);

        assert_eq!(auth.username(), "agent-user");
        assert_eq!(
            auth,
            BackendAuth::Agent {
                username: "agent-user".to_owned(),
                source: AgentSource::Auto,
                key_hint: Some("id_ed25519".to_owned()),
            }
        );
    }

    #[test]
    fn backend_auth_preserves_certificate_secret_references() {
        let profile = AuthProfile::Certificate {
            username: "cert-user".to_owned(),
            key: SecretRef("key:cert".to_owned()),
            passphrase: Some(SecretRef("passphrase:cert".to_owned())),
            certificate: SecretRef("cert:deploy".to_owned()),
        };

        let auth = BackendAuth::from(&profile);

        assert_eq!(auth.username(), "cert-user");
        assert_eq!(
            auth,
            BackendAuth::Certificate {
                username: "cert-user".to_owned(),
                key: SecretRef("key:cert".to_owned()),
                passphrase: Some(SecretRef("passphrase:cert".to_owned())),
                certificate: SecretRef("cert:deploy".to_owned()),
            }
        );
    }
}
