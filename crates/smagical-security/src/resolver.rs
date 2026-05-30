//! SSH 认证材料解析。
//!
//! 解析器是 SecretRef 变成临时明文的唯一入口。调用方拿到 `ResolvedAuth` 后应尽快交给
//! SSH 执行器使用，不应把它写入日志、配置或持久化存储。

use crate::BackendAuth;
use smagical_core::AgentSource;

use super::{SecretStore, SecurityError};

/// 已解析的临时认证材料。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedAuth {
    /// 已解析的密码认证。
    Password { username: String, password: String },
    /// 已解析的私钥认证。
    Key {
        username: String,
        private_key: String,
        passphrase: Option<String>,
    },
    /// agent 认证不含明文，只保留来源和 key hint。
    Agent {
        username: String,
        source: AgentSource,
        key_hint: Option<String>,
    },
    /// 已解析的证书认证。
    Certificate {
        username: String,
        private_key: String,
        passphrase: Option<String>,
        certificate: String,
    },
}

/// 把后端认证引用解析为临时明文材料。
pub struct AuthResolver<'a, S: SecretStore> {
    /// 抽象秘密存储，后续可以替换为加密 SQLite、系统钥匙串或内存测试存储。
    store: &'a S,
}

impl<'a, S: SecretStore> AuthResolver<'a, S> {
    /// 创建认证解析器。
    pub fn new(store: &'a S) -> Self {
        Self { store }
    }

    /// 解析认证引用。
    pub fn resolve(&self, auth: &BackendAuth) -> Result<ResolvedAuth, SecurityError> {
        // 每个 SecretRef 都显式读取；可选口令使用 transpose 保留 None 和错误的区别。
        match auth {
            BackendAuth::Password { username, secret } => Ok(ResolvedAuth::Password {
                username: username.clone(),
                password: self.store.get_secret(secret)?,
            }),
            BackendAuth::Key {
                username,
                key,
                passphrase,
            } => Ok(ResolvedAuth::Key {
                username: username.clone(),
                private_key: self.store.get_secret(key)?,
                passphrase: passphrase
                    .as_ref()
                    .map(|reference| self.store.get_secret(reference))
                    .transpose()?,
            }),
            BackendAuth::Agent {
                username,
                source,
                key_hint,
            } => Ok(ResolvedAuth::Agent {
                username: username.clone(),
                source: source.clone(),
                key_hint: key_hint.clone(),
            }),
            BackendAuth::Certificate {
                username,
                key,
                passphrase,
                certificate,
            } => Ok(ResolvedAuth::Certificate {
                username: username.clone(),
                private_key: self.store.get_secret(key)?,
                passphrase: passphrase
                    .as_ref()
                    .map(|reference| self.store.get_secret(reference))
                    .transpose()?,
                certificate: self.store.get_secret(certificate)?,
            }),
        }
    }
}

impl ResolvedAuth {
    /// 返回认证用户名。
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
    use crate::MemorySecretStore;
    use smagical_core::SecretRef;

    #[test]
    fn resolver_reads_password_from_secret_store() {
        let mut store = MemorySecretStore::new();
        let reference = SecretRef("password:root".to_owned());
        store
            .set_secret(&reference, "s3cret")
            .expect("内存凭据应该可以写入");
        let resolver = AuthResolver::new(&store);

        let resolved = resolver
            .resolve(&BackendAuth::Password {
                username: "root".to_owned(),
                secret: reference,
            })
            .expect("密码认证应该可以解析");

        assert_eq!(resolved.username(), "root");
        assert_eq!(
            resolved,
            ResolvedAuth::Password {
                username: "root".to_owned(),
                password: "s3cret".to_owned(),
            }
        );
    }

    #[test]
    fn resolver_reads_key_and_optional_passphrase() {
        let mut store = MemorySecretStore::new();
        let key_ref = SecretRef("key:deploy".to_owned());
        let passphrase_ref = SecretRef("passphrase:deploy".to_owned());
        store
            .set_secret(&key_ref, "PRIVATE KEY")
            .expect("私钥应该可以写入");
        store
            .set_secret(&passphrase_ref, "phrase")
            .expect("私钥口令应该可以写入");
        let resolver = AuthResolver::new(&store);

        let resolved = resolver
            .resolve(&BackendAuth::Key {
                username: "deploy".to_owned(),
                key: key_ref,
                passphrase: Some(passphrase_ref),
            })
            .expect("私钥认证应该可以解析");

        assert_eq!(
            resolved,
            ResolvedAuth::Key {
                username: "deploy".to_owned(),
                private_key: "PRIVATE KEY".to_owned(),
                passphrase: Some("phrase".to_owned()),
            }
        );
    }

    #[test]
    fn resolver_keeps_missing_key_passphrase_as_none() {
        let mut store = MemorySecretStore::new();
        let key_ref = SecretRef("key:deploy".to_owned());
        store
            .set_secret(&key_ref, "PRIVATE KEY")
            .expect("私钥应该可以写入");
        let resolver = AuthResolver::new(&store);

        let resolved = resolver
            .resolve(&BackendAuth::Key {
                username: "deploy".to_owned(),
                key: key_ref,
                passphrase: None,
            })
            .expect("无口令私钥认证应该可以解析");

        assert_eq!(
            resolved,
            ResolvedAuth::Key {
                username: "deploy".to_owned(),
                private_key: "PRIVATE KEY".to_owned(),
                passphrase: None,
            }
        );
    }

    #[test]
    fn resolver_reads_certificate_key_and_optional_passphrase() {
        let mut store = MemorySecretStore::new();
        let key_ref = SecretRef("key:cert".to_owned());
        let passphrase_ref = SecretRef("passphrase:cert".to_owned());
        let certificate_ref = SecretRef("cert:cert".to_owned());
        store
            .set_secret(&key_ref, "PRIVATE KEY")
            .expect("私钥应该可以写入");
        store
            .set_secret(&passphrase_ref, "phrase")
            .expect("证书私钥口令应该可以写入");
        store
            .set_secret(&certificate_ref, "CERT")
            .expect("证书应该可以写入");
        let resolver = AuthResolver::new(&store);

        let resolved = resolver
            .resolve(&BackendAuth::Certificate {
                username: "cert-user".to_owned(),
                key: key_ref,
                passphrase: Some(passphrase_ref),
                certificate: certificate_ref,
            })
            .expect("证书认证应该可以解析");

        assert_eq!(
            resolved,
            ResolvedAuth::Certificate {
                username: "cert-user".to_owned(),
                private_key: "PRIVATE KEY".to_owned(),
                passphrase: Some("phrase".to_owned()),
                certificate: "CERT".to_owned(),
            }
        );
    }

    #[test]
    fn resolver_keeps_missing_certificate_passphrase_as_none() {
        let mut store = MemorySecretStore::new();
        let key_ref = SecretRef("key:cert".to_owned());
        let certificate_ref = SecretRef("cert:cert".to_owned());
        store
            .set_secret(&key_ref, "PRIVATE KEY")
            .expect("私钥应该可以写入");
        store
            .set_secret(&certificate_ref, "CERT")
            .expect("证书应该可以写入");
        let resolver = AuthResolver::new(&store);

        let resolved = resolver
            .resolve(&BackendAuth::Certificate {
                username: "cert-user".to_owned(),
                key: key_ref,
                passphrase: None,
                certificate: certificate_ref,
            })
            .expect("无口令证书认证应该可以解析");

        assert_eq!(
            resolved,
            ResolvedAuth::Certificate {
                username: "cert-user".to_owned(),
                private_key: "PRIVATE KEY".to_owned(),
                passphrase: None,
                certificate: "CERT".to_owned(),
            }
        );
    }

    #[test]
    fn resolver_keeps_agent_auth_secretless() {
        let store = MemorySecretStore::new();
        let resolver = AuthResolver::new(&store);

        let resolved = resolver
            .resolve(&BackendAuth::Agent {
                username: "agent-user".to_owned(),
                source: AgentSource::Auto,
                key_hint: Some("id_ed25519".to_owned()),
            })
            .expect("agent 认证不需要读取凭据");

        assert_eq!(
            resolved,
            ResolvedAuth::Agent {
                username: "agent-user".to_owned(),
                source: AgentSource::Auto,
                key_hint: Some("id_ed25519".to_owned()),
            }
        );
    }

    #[test]
    fn resolver_reports_missing_secret() {
        let store = MemorySecretStore::new();
        let missing = SecretRef("missing".to_owned());
        let resolver = AuthResolver::new(&store);

        let error = resolver
            .resolve(&BackendAuth::Password {
                username: "root".to_owned(),
                secret: missing.clone(),
            })
            .expect_err("缺失凭据应该返回错误");

        assert_eq!(error, SecurityError::MissingSecret(missing));
    }

    #[test]
    fn resolver_reports_missing_key_passphrase_secret() {
        let mut store = MemorySecretStore::new();
        let key_ref = SecretRef("key:deploy".to_owned());
        let missing_passphrase = SecretRef("passphrase:missing".to_owned());
        store
            .set_secret(&key_ref, "PRIVATE KEY")
            .expect("私钥应该可以写入");
        let resolver = AuthResolver::new(&store);

        let error = resolver
            .resolve(&BackendAuth::Key {
                username: "deploy".to_owned(),
                key: key_ref,
                passphrase: Some(missing_passphrase.clone()),
            })
            .expect_err("缺失私钥口令应该返回错误");

        assert_eq!(error, SecurityError::MissingSecret(missing_passphrase));
    }

    #[test]
    fn resolver_reports_missing_certificate_secret() {
        let mut store = MemorySecretStore::new();
        let key_ref = SecretRef("key:cert".to_owned());
        let missing_certificate = SecretRef("cert:missing".to_owned());
        store
            .set_secret(&key_ref, "PRIVATE KEY")
            .expect("私钥应该可以写入");
        let resolver = AuthResolver::new(&store);

        let error = resolver
            .resolve(&BackendAuth::Certificate {
                username: "cert-user".to_owned(),
                key: key_ref,
                passphrase: None,
                certificate: missing_certificate.clone(),
            })
            .expect_err("缺失证书应该返回错误");

        assert_eq!(error, SecurityError::MissingSecret(missing_certificate));
    }
}
