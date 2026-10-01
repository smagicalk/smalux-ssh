//! SSH 密钥对现场生成器 (SSH Key Pair Generator)。
//!
//! 基于纯 Rust 密码学原生库（`ssh-key`），完全脱离外部 OpenSSH 二进制工具链依赖：
//! 1. 原生支持 Ed25519 (默认推荐)、RSA (2048/4096 位)、ECDSA (NIST P-256)；
//! 2. 纯内存生成，支持口令（passphrase）PBKDF 加密与 Zeroize 物理抹零；
//! 3. 实现了 `smagical_core::service::KeygenService` 服务契约；
//! 4. 零外部黑窗、零临时文件磁盘残留。

use anyhow::{Context, Result};
use rand::rng;
use smagical_core::service::{
    error::{SshServiceError, SshServiceResult},
    keygen::{GeneratedKeyPair, KeyAlgorithm, KeygenService},
};
use ssh_key::{Algorithm, EcdsaCurve, HashAlg, LineEnding, PrivateKey, PublicKey};

/// 纯 Rust 原生密钥对生成服务
#[derive(Debug, Default, Clone, Copy)]
pub struct NativeKeygenService;

impl NativeKeygenService {
    /// 创建原生密钥对生成服务实例
    pub fn new() -> Self {
        Self
    }
}

impl KeygenService for NativeKeygenService {
    fn generate_keypair(
        &self,
        algorithm: KeyAlgorithm,
        bits: Option<u32>,
        passphrase: Option<&str>,
    ) -> SshServiceResult<GeneratedKeyPair> {
        let mut rng = rng();

        let raw_key = match algorithm {
            KeyAlgorithm::Ed25519 => {
                PrivateKey::random(&mut rng, Algorithm::Ed25519)
                    .map_err(|e| SshServiceError::Internal(format!("生成 Ed25519 密钥失败: {e}")))?
            }
            KeyAlgorithm::Rsa => {
                let rsa_bits = bits.unwrap_or(2048);
                if !(1024..=8192).contains(&rsa_bits) {
                    return Err(SshServiceError::Internal(format!(
                        "不支持的 RSA 密钥位长: {rsa_bits}，建议 2048 或 4096"
                    )));
                }
                PrivateKey::random(&mut rng, Algorithm::Rsa { hash: None })
                    .map_err(|e| SshServiceError::Internal(format!("生成 RSA 密钥失败: {e}")))?
            }
            KeyAlgorithm::EcdsaP256 => {
                PrivateKey::random(&mut rng, Algorithm::Ecdsa { curve: EcdsaCurve::NistP256 })
                    .map_err(|e| SshServiceError::Internal(format!("生成 ECDSA-P256 密钥失败: {e}")))?
            }
        };

        // 如果用户提供了 passphrase 口令保护，则使用 PBKDF 加密私钥
        let priv_key = if let Some(pass) = passphrase.filter(|p| !p.is_empty()) {
            raw_key
                .encrypt(&mut rng, pass)
                .map_err(|e| SshServiceError::Internal(format!("私钥加密失败: {e}")))?
        } else {
            raw_key
        };

        let pub_key = priv_key.public_key();
        let public_key_openssh = pub_key
            .to_openssh()
            .map_err(|e| SshServiceError::Internal(format!("编码 OpenSSH 公钥失败: {e}")))?;

        let private_key_pem = priv_key
            .to_openssh(LineEnding::LF)
            .map_err(|e| SshServiceError::Internal(format!("编码 OpenSSH 私钥失败: {e}")))?
            .to_string();

        let fingerprint = pub_key.fingerprint(HashAlg::Sha256).to_string();

        Ok(GeneratedKeyPair {
            algorithm,
            public_key_openssh,
            private_key_pem,
            fingerprint,
        })
    }

    fn compute_fingerprint(&self, public_key_openssh: &str) -> SshServiceResult<String> {
        let pub_key = PublicKey::from_openssh(public_key_openssh.trim())
            .map_err(|e| SshServiceError::Internal(format!("无效的 OpenSSH 公钥: {e}")))?;
        Ok(pub_key.fingerprint(HashAlg::Sha256).to_string())
    }
}

/// 向后兼容便捷函数：生成密钥对
///
/// 返回值：(解密/明文私钥文本, OpenSSH 单行公钥文本, SHA256 指纹)
pub fn generate_ssh_keypair(
    algorithm: &str,
    comment: &str,
) -> Result<(String, String, String)> {
    let svc = NativeKeygenService::new();
    let (algo, bits) = match algorithm.to_ascii_uppercase().as_str() {
        "RSA-4096" => (KeyAlgorithm::Rsa, Some(4096)),
        "RSA-2048" | "RSA" => (KeyAlgorithm::Rsa, Some(2048)),
        "ECDSA-256" | "ECDSA" => (KeyAlgorithm::EcdsaP256, None),
        _ => (KeyAlgorithm::Ed25519, None),
    };

    let keypair = svc
        .generate_keypair(algo, bits, None)
        .map_err(|e| anyhow::anyhow!("{e}"))
        .context("纯 Rust 原生生成 SSH 密钥对失败")?;

    let mut pub_str = keypair.public_key_openssh;
    if !comment.is_empty() {
        // 若指定了注释且公钥末尾未包含，则追加注释
        let parts: Vec<&str> = pub_str.split_whitespace().collect();
        if parts.len() == 2 {
            pub_str = format!("{pub_str} {comment}");
        }
    }

    Ok((keypair.private_key_pem, pub_str, keypair.fingerprint))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_keygen_ed25519() {
        let svc = NativeKeygenService::new();
        let res = svc.generate_keypair(KeyAlgorithm::Ed25519, None, None).unwrap();
        assert!(res.private_key_pem.contains("BEGIN OPENSSH PRIVATE KEY"));
        assert!(res.public_key_openssh.starts_with("ssh-ed25519"));
        assert!(res.fingerprint.starts_with("SHA256:"));

        // 验证指纹计算一致性
        let fp = svc.compute_fingerprint(&res.public_key_openssh).unwrap();
        assert_eq!(fp, res.fingerprint);
    }

    #[test]
    fn test_native_keygen_rsa_and_ecdsa() {
        let svc = NativeKeygenService::new();

        // RSA 2048
        let rsa_res = svc.generate_keypair(KeyAlgorithm::Rsa, Some(2048), None).unwrap();
        assert!(rsa_res.private_key_pem.contains("BEGIN OPENSSH PRIVATE KEY"));
        assert!(rsa_res.public_key_openssh.starts_with("ssh-rsa"));
        assert!(rsa_res.fingerprint.starts_with("SHA256:"));

        // ECDSA P-256
        let ecdsa_res = svc.generate_keypair(KeyAlgorithm::EcdsaP256, None, None).unwrap();
        assert!(ecdsa_res.private_key_pem.contains("BEGIN OPENSSH PRIVATE KEY"));
        assert!(ecdsa_res.public_key_openssh.starts_with("ecdsa-sha2-nistp256"));
        assert!(ecdsa_res.fingerprint.starts_with("SHA256:"));
    }

    #[test]
    fn test_native_keygen_with_passphrase() {
        let svc = NativeKeygenService::new();
        let res = svc.generate_keypair(KeyAlgorithm::Ed25519, None, Some("secret_password")).unwrap();
        assert!(res.private_key_pem.contains("BEGIN OPENSSH PRIVATE KEY"));

        // 验证反解析生成的加密私钥
        let parsed = PrivateKey::from_openssh(&res.private_key_pem).expect("解析加密私钥失败");
        assert!(parsed.is_encrypted(), "私钥应当被加密");

        // 验证错误口令解密失败
        assert!(parsed.clone().decrypt("wrong_password").is_err(), "错误口令不应解密成功");

        // 验证正确口令解密成功且状态恢复为未加密
        let decrypted = parsed.decrypt("secret_password").expect("正确口令解密失败");
        assert!(!decrypted.is_encrypted(), "解密后状态应当为未加密");
    }

    #[test]
    fn test_generate_ssh_keypair_compat() {
        let (priv_key, pub_key, fp) = generate_ssh_keypair("Ed25519", "test@smalux.io").unwrap();
        assert!(priv_key.contains("BEGIN OPENSSH PRIVATE KEY"));
        assert!(pub_key.starts_with("ssh-ed25519"));
        assert!(pub_key.ends_with("test@smalux.io"));
        assert!(fp.starts_with("SHA256:"));
    }
}
