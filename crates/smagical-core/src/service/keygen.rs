//! SSH 密钥对生成与格式转换契约。

use crate::service::error::SshServiceResult;

/// 支持生成的密钥算法类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAlgorithm {
    /// Ed25519 高安全性椭圆曲线算法
    Ed25519,
    /// 经典 RSA 算法 (支持 2048 / 4096 位)
    Rsa,
    /// ECDSA 椭圆曲线算法 (NIST P-256)
    EcdsaP256,
}

/// 密钥对生成结果载荷
#[derive(Debug, Clone)]
pub struct GeneratedKeyPair {
    /// 算法类型
    pub algorithm: KeyAlgorithm,
    /// OpenSSH 格式公钥单行文本 (例如 "ssh-ed25519 AAAAC3... user@host")
    pub public_key_openssh: String,
    /// PEM 格式私钥文本内容
    pub private_key_pem: String,
    /// SHA256 公钥指纹 (例如 "SHA256:xxxx...")
    pub fingerprint: String,
}

/// 密钥对生成服务契约
pub trait KeygenService: Send + Sync {
    /// 原生纯 Rust 生成指定算法的密钥对
    fn generate_keypair(
        &self,
        algorithm: KeyAlgorithm,
        bits: Option<u32>,
        passphrase: Option<&str>,
    ) -> SshServiceResult<GeneratedKeyPair>;

    /// 计算公钥的标准 SHA256 格式指纹
    fn compute_fingerprint(&self, public_key_openssh: &str) -> SshServiceResult<String>;
}
