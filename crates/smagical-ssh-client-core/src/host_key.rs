//! SSH 主机密钥校验策略。

use russh::keys::{HashAlg, PublicKey};
use smagical_core::{HostKeyVerification, KeyAlgorithm, KnownHostEntry};

/// 主机密钥校验策略。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostKeyPolicy {
    /// 明确允许未知主机密钥，仅用于测试或用户显式放宽校验。
    AcceptAny,
    /// 只允许已信任的 Known Hosts 记录。
    KnownHosts(Vec<KnownHostEntry>),
}

impl Default for HostKeyPolicy {
    fn default() -> Self {
        Self::KnownHosts(Vec::new())
    }
}

impl HostKeyPolicy {
    /// 校验服务端主机密钥并返回是否允许连接。
    pub fn check(&self, host: &str, port: u16, public_key: &PublicKey) -> HostKeyCheck {
        let fingerprint = host_key_fingerprint(public_key);

        match self {
            Self::AcceptAny => HostKeyCheck {
                host: host.to_owned(),
                port,
                key_algorithm: host_key_algorithm(public_key),
                verification: HostKeyVerification::Unknown,
                accepted: true,
                fingerprint,
            },
            Self::KnownHosts(entries) => {
                let verification = entries
                    .iter()
                    .find(|entry| entry.host == host && entry.port == port)
                    .map(|entry| entry.verify(host, port, &fingerprint))
                    .unwrap_or(HostKeyVerification::Unknown);

                HostKeyCheck {
                    host: host.to_owned(),
                    port,
                    key_algorithm: host_key_algorithm(public_key),
                    accepted: matches!(verification, HostKeyVerification::Trusted),
                    verification,
                    fingerprint,
                }
            }
        }
    }
}

/// 单次主机密钥校验结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostKeyCheck {
    pub host: String,
    pub port: u16,
    pub key_algorithm: KeyAlgorithm,
    pub verification: HostKeyVerification,
    pub accepted: bool,
    pub fingerprint: String,
}

/// 返回服务端主机密钥的 SHA256 指纹。
pub fn host_key_fingerprint(public_key: &PublicKey) -> String {
    public_key.fingerprint(HashAlg::Sha256).to_string()
}

/// 返回服务端主机密钥算法。
pub fn host_key_algorithm(public_key: &PublicKey) -> KeyAlgorithm {
    KeyAlgorithm::from_ssh_algorithm(public_key.algorithm().as_str())
}
