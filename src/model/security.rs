//! 凭据元数据和 Known Hosts 校验模型。

use serde::{Deserialize, Serialize};

use super::SecretRef;

/// 凭据元数据。
///
/// 只保存可展示和可索引的信息；明文密码、私钥口令等敏感内容必须通过 `SecretRef` 间接读取。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialMetadata {
    pub name: String,
    pub kind: CredentialKind,
    pub username: Option<String>,
    pub secret: Option<SecretRef>,
    pub key_algorithm: Option<KeyAlgorithm>,
    pub fingerprint: Option<String>,
}

/// 凭据类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CredentialKind {
    Password,
    PrivateKey,
    Agent,
    Certificate,
}

/// SSH 密钥算法。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyAlgorithm {
    Ed25519,
    Rsa,
    Ecdsa,
    Unknown(String),
}

/// Known Hosts 记录。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownHostEntry {
    pub host: String,
    pub port: u16,
    pub key_algorithm: KeyAlgorithm,
    pub fingerprint: String,
    pub trusted: bool,
}

impl KnownHostEntry {
    /// 判断远端主机指纹是否与本地记录匹配。
    pub fn verify(&self, host: &str, port: u16, fingerprint: &str) -> HostKeyVerification {
        if self.host != host || self.port != port {
            return HostKeyVerification::Unknown;
        }

        if self.fingerprint == fingerprint && self.trusted {
            HostKeyVerification::Trusted
        } else if self.fingerprint == fingerprint {
            HostKeyVerification::Untrusted
        } else {
            HostKeyVerification::Mismatch {
                expected: self.fingerprint.clone(),
                actual: fingerprint.to_owned(),
            }
        }
    }
}

/// 主机密钥校验结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostKeyVerification {
    Trusted,
    Untrusted,
    Unknown,
    Mismatch { expected: String, actual: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_host_entry_verifies_trusted_untrusted_unknown_and_mismatch() {
        let entry = KnownHostEntry {
            host: "example.com".to_owned(),
            port: 22,
            key_algorithm: KeyAlgorithm::Ed25519,
            fingerprint: "SHA256:trusted".to_owned(),
            trusted: true,
        };
        let untrusted = KnownHostEntry {
            trusted: false,
            ..entry.clone()
        };

        assert_eq!(
            entry.verify("example.com", 22, "SHA256:trusted"),
            HostKeyVerification::Trusted
        );
        assert_eq!(
            untrusted.verify("example.com", 22, "SHA256:trusted"),
            HostKeyVerification::Untrusted
        );
        assert_eq!(
            entry.verify("other.example.com", 22, "SHA256:trusted"),
            HostKeyVerification::Unknown
        );
        assert_eq!(
            entry.verify("example.com", 22, "SHA256:changed"),
            HostKeyVerification::Mismatch {
                expected: "SHA256:trusted".to_owned(),
                actual: "SHA256:changed".to_owned(),
            }
        );
    }
}
