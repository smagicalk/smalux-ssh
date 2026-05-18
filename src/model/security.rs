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

impl KeyAlgorithm {
    /// 从 OpenSSH 算法标识转换为内部展示分类。
    pub fn from_ssh_algorithm(algorithm: &str) -> Self {
        if algorithm.contains("ed25519") {
            Self::Ed25519
        } else if algorithm.contains("rsa") {
            Self::Rsa
        } else if algorithm.contains("ecdsa") {
            Self::Ecdsa
        } else {
            Self::Unknown(algorithm.to_owned())
        }
    }
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
    /// 创建一个等待用户确认的 Known Hosts 候选记录。
    pub fn untrusted(
        host: impl Into<String>,
        port: u16,
        key_algorithm: KeyAlgorithm,
        fingerprint: impl Into<String>,
    ) -> Self {
        Self {
            host: host.into(),
            port,
            key_algorithm,
            fingerprint: fingerprint.into(),
            trusted: false,
        }
    }

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
    fn key_algorithm_maps_common_openssh_names() {
        assert_eq!(
            KeyAlgorithm::from_ssh_algorithm("ssh-ed25519"),
            KeyAlgorithm::Ed25519
        );
        assert_eq!(
            KeyAlgorithm::from_ssh_algorithm("rsa-sha2-256"),
            KeyAlgorithm::Rsa
        );
        assert_eq!(
            KeyAlgorithm::from_ssh_algorithm("ecdsa-sha2-nistp256"),
            KeyAlgorithm::Ecdsa
        );
        assert_eq!(
            KeyAlgorithm::from_ssh_algorithm("ssh-dss"),
            KeyAlgorithm::Unknown("ssh-dss".to_owned())
        );
    }

    #[test]
    fn untrusted_known_host_entry_keeps_candidate_fingerprint() {
        let entry =
            KnownHostEntry::untrusted("example.com", 22, KeyAlgorithm::Ed25519, "SHA256:new");

        assert_eq!(entry.host, "example.com");
        assert_eq!(entry.port, 22);
        assert_eq!(entry.fingerprint, "SHA256:new");
        assert!(!entry.trusted);
    }

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
