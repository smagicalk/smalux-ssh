//! 凭据元数据和 Known Hosts 校验模型。

use serde::{Deserialize, Serialize};

use crate::ids::{CredentialGroupId, CredentialId, SecretRef};

/// 密钥分组。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialGroup {
    pub id: CredentialGroupId,
    pub name: String,
    #[serde(default = "default_credential_group_kind")]
    pub kind: CredentialKind,
    pub parent_id: Option<CredentialGroupId>,
    pub sort_order: i32,
}

/// 凭据元数据。
///
/// 只保存可展示和可索引的信息；明文密码、私钥口令等敏感内容必须通过 `SecretRef` 间接读取。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialMetadata {
    #[serde(default = "default_credential_id")]
    pub id: CredentialId,
    pub name: String,
    pub kind: CredentialKind,
    #[serde(default)]
    pub group_id: Option<CredentialGroupId>,
    pub username: Option<String>,
    pub secret: Option<SecretRef>,
    pub key_algorithm: Option<KeyAlgorithm>,
    pub fingerprint: Option<String>,
}

fn default_credential_id() -> CredentialId {
    CredentialId(uuid::Uuid::new_v4())
}

/// 凭据内容解析缓存。
///
/// `secrets` 中的原始 payload 仍是唯一事实来源；这个结构只保存可重建的展示、搜索和排序信息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialInspection {
    pub credential_id: CredentialId,
    pub kind: CredentialKind,
    pub payload_hash: String,
    pub parser_version: i32,
    pub parse_error: Option<String>,
    pub algorithm: Option<KeyAlgorithm>,
    pub fingerprint: Option<String>,
    pub public_key: Option<String>,
    pub comment: Option<String>,
    pub encrypted: Option<bool>,
    pub password_length: Option<usize>,
    pub certificate: Option<CertificateInspection>,
}

/// OpenSSH 证书解析缓存。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CertificateInspection {
    pub cert_type: Option<String>,
    pub serial: Option<u64>,
    pub key_id: Option<String>,
    pub principals: Vec<String>,
    pub valid_after_unix_secs: Option<u64>,
    pub valid_before_unix_secs: Option<u64>,
    pub ca_fingerprint: Option<String>,
    pub subject_fingerprint: Option<String>,
    pub critical_options_json: Option<String>,
    pub extensions_json: Option<String>,
}

/// 凭据类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CredentialKind {
    Password,
    PrivateKey,
    Agent,
    Certificate,
}

fn default_credential_group_kind() -> CredentialKind {
    CredentialKind::PrivateKey
}

/// 敏感材料类型。
///
/// 和 `CredentialKind` 分开建模，是因为 ssh-agent 只有元数据和引用，通常没有需要本地保存的
/// secret payload；而私钥口令这类材料也不等同于一个可展示凭据。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretMaterialKind {
    Password,
    PrivateKey,
    Passphrase,
    Certificate,
    Unknown(String),
}

/// 本地安全存储中的 secret 记录。
///
/// `encryption_version = 0` 表示当前未启用主密码加密，`encrypted_payload` 直接保存原始字节。
/// 后续设置页接入主密码后，可以升级为非 0 版本并填充 kdf/salt/nonce，而不改变外层引用关系。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretRecord {
    pub secret_ref: SecretRef,
    pub kind: SecretMaterialKind,
    pub encryption_version: i32,
    pub kdf: Option<String>,
    pub kdf_params_toml: Option<String>,
    pub salt: Option<Vec<u8>>,
    pub nonce: Option<Vec<u8>>,
    pub encrypted_payload: Option<Vec<u8>>,
    pub external_store: Option<String>,
    pub external_key: Option<String>,
}

impl SecretRecord {
    /// 用当前未加密存储格式创建本地 payload。
    pub fn local_plaintext(
        secret_ref: SecretRef,
        kind: SecretMaterialKind,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            secret_ref,
            kind,
            encryption_version: 0,
            kdf: None,
            kdf_params_toml: None,
            salt: None,
            nonce: None,
            encrypted_payload: Some(payload),
            external_store: None,
            external_key: None,
        }
    }

    /// 用外部凭据库引用创建占位记录。
    pub fn external(
        secret_ref: SecretRef,
        kind: SecretMaterialKind,
        store: impl Into<String>,
        key: impl Into<String>,
    ) -> Self {
        Self {
            secret_ref,
            kind,
            encryption_version: 0,
            kdf: None,
            kdf_params_toml: None,
            salt: None,
            nonce: None,
            encrypted_payload: None,
            external_store: Some(store.into()),
            external_key: Some(key.into()),
        }
    }
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
