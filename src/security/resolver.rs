//! SSH 认证材料解析。

use crate::backend::BackendAuth;

use super::{SecretStore, SecurityError};

/// 已解析的临时认证材料。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedAuth {
    Password {
        username: String,
        password: String,
    },
    Key {
        username: String,
        private_key: String,
        passphrase: Option<String>,
    },
    Agent {
        username: String,
        key_hint: Option<String>,
    },
    Certificate {
        username: String,
        private_key: String,
        passphrase: Option<String>,
        certificate: String,
    },
}

/// 把后端认证引用解析为临时明文材料。
pub struct AuthResolver<'a, S: SecretStore> {
    store: &'a S,
}

impl<'a, S: SecretStore> AuthResolver<'a, S> {
    /// 创建认证解析器。
    pub fn new(store: &'a S) -> Self {
        Self { store }
    }

    /// 解析认证引用。
    pub fn resolve(&self, auth: &BackendAuth) -> Result<ResolvedAuth, SecurityError> {
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
            BackendAuth::Agent { username, key_hint } => Ok(ResolvedAuth::Agent {
                username: username.clone(),
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
    use crate::model::SecretRef;
    use crate::security::MemorySecretStore;

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
    fn resolver_keeps_agent_auth_secretless() {
        let store = MemorySecretStore::new();
        let resolver = AuthResolver::new(&store);

        let resolved = resolver
            .resolve(&BackendAuth::Agent {
                username: "agent-user".to_owned(),
                key_hint: Some("id_ed25519".to_owned()),
            })
            .expect("agent 认证不需要读取凭据");

        assert_eq!(
            resolved,
            ResolvedAuth::Agent {
                username: "agent-user".to_owned(),
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
}
