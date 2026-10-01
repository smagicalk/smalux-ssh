//! 安全加密与密钥衍生服务层 (Cryptographic & KDF Service)。
//!
//! 基于 Argon2id (RFC 9106) 内存硬化 KDF 与 AES-256-GCM 认证加密构建。
//! 支持金丝雀魔数校验 (Canary Verification)、信封加密 (Envelope Encryption) 与密文版本追踪。

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use anyhow::{anyhow, Context, Result};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use rand::RngCore;

pub mod vault;
pub use vault::VaultManager;

/// 金丝雀固定魔数校验常量
pub const CANARY_MAGIC: &str = "SMALUX_SSH_VAULT_CANARY_V1";

/// Argon2id 推荐安全参数 (RFC 9106)
pub const DEFAULT_ARGON2_MEMORY_KIB: u32 = 65536; // 64 MB
pub const DEFAULT_ARGON2_ITERATIONS: u32 = 3;     // 3 轮迭代
pub const DEFAULT_ARGON2_PARALLELISM: u32 = 4;    // 4 并发线程

/// 加密与密码派生服务门面
pub struct CryptoService;

impl CryptoService {
    /// 生成 32 字节真随机 Salt
    pub fn generate_salt() -> [u8; 32] {
        let mut salt = [0u8; 32];
        OsRng.fill_bytes(&mut salt);
        salt
    }

    /// 生成 32 字节真随机数据加密密钥 (DEK)
    pub fn generate_dek() -> [u8; 32] {
        let mut dek = [0u8; 32];
        OsRng.fill_bytes(&mut dek);
        dek
    }

    /// 使用 Argon2id 将用户密码与 Salt 派生为 256 位 (32 字节) MasterKey
    pub fn derive_master_key(
        password: &str,
        salt: &[u8],
        memory_kib: u32,
        iterations: u32,
        parallelism: u32,
    ) -> Result<[u8; 32]> {
        let params = Params::new(memory_kib, iterations, parallelism, Some(32))
            .map_err(|e| anyhow!("Argon2 参数非法: {}", e))?;
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);

        let mut master_key = [0u8; 32];
        argon2
            .hash_password_into(password.as_bytes(), salt, &mut master_key)
            .map_err(|e| anyhow!("Argon2 密钥派生失败: {}", e))?;

        Ok(master_key)
    }

    /// 使用指定 256 位密钥通过 AES-256-GCM 加密明文字节
    ///
    /// 输出密文格式：`enc:v<version>:<base64_nonce>:<base64_ciphertext_with_tag>`
    pub fn encrypt(key: &[u8; 32], version: u32, plaintext: &[u8]) -> Result<String> {
        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| anyhow!("初始化 AES-256-GCM 密码机失败: {}", e))?;

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| anyhow!("AES-256-GCM 加密失败: {}", e))?;

        let nonce_b64 = BASE64.encode(nonce_bytes);
        let cipher_b64 = BASE64.encode(ciphertext);

        Ok(format!("enc:v{}:{}:{}", version, nonce_b64, cipher_b64))
    }

    /// 使用指定 256 位密钥通过 AES-256-GCM 解密密文格式字符串
    pub fn decrypt(key: &[u8; 32], encrypted_str: &str) -> Result<(u32, Vec<u8>)> {
        if !encrypted_str.starts_with("enc:v") {
            return Err(anyhow!("非合法的加密密文前缀"));
        }

        let parts: Vec<&str> = encrypted_str.split(':').collect();
        if parts.len() != 4 {
            return Err(anyhow!("密文分段不符合规范 enc:v<ver>:<nonce>:<data>"));
        }

        let version_str = parts[1].strip_prefix('v').unwrap_or(parts[1]);
        let version: u32 = version_str.parse().context("解析密文版本号失败")?;

        let nonce_bytes = BASE64.decode(parts[2]).context("Base64 解码 Nonce 失败")?;
        if nonce_bytes.len() != 12 {
            return Err(anyhow!("Nonce 长度不符合 12 字节标准"));
        }
        let nonce = Nonce::from_slice(&nonce_bytes);

        let cipher_bytes = BASE64.decode(parts[3]).context("Base64 解码 Ciphertext 失败")?;

        let cipher = Aes256Gcm::new_from_slice(key)
            .map_err(|e| anyhow!("初始化 AES-256-GCM 密码机失败: {}", e))?;

        let plaintext = cipher
            .decrypt(nonce, cipher_bytes.as_ref())
            .map_err(|_| anyhow!("解密校验失败 (密钥错误或密文被篡改)"))?;

        Ok((version, plaintext))
    }

    /// 使用 MasterKey 加密固定金丝雀魔数串
    pub fn create_canary(master_key: &[u8; 32], version: u32) -> Result<String> {
        Self::encrypt(master_key, version, CANARY_MAGIC.as_bytes())
    }

    /// 使用候选 MasterKey 校验金丝雀魔数串是否能正常解密且内容相符
    pub fn verify_canary(candidate_key: &[u8; 32], canary_ciphertext: &str) -> bool {
        match Self::decrypt(candidate_key, canary_ciphertext) {
            Ok((_, plaintext)) => plaintext == CANARY_MAGIC.as_bytes(),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_argon2id_derivation_and_canary_verification() {
        let password = "MySecureSuperPassword#2026";
        let salt = CryptoService::generate_salt();

        let master_key = CryptoService::derive_master_key(
            password,
            &salt,
            DEFAULT_ARGON2_MEMORY_KIB,
            DEFAULT_ARGON2_ITERATIONS,
            DEFAULT_ARGON2_PARALLELISM,
        )
        .expect("KDF 派生应当成功");

        let canary = CryptoService::create_canary(&master_key, 1).expect("创建金丝雀密文应当成功");
        assert!(canary.starts_with("enc:v1:"));

        // 验证正确密码
        let is_valid = CryptoService::verify_canary(&master_key, &canary);
        assert!(is_valid, "正确 MasterKey 应当验证成功");

        // 验证错误密码
        let wrong_key = CryptoService::derive_master_key(
            "WrongPassword",
            &salt,
            DEFAULT_ARGON2_MEMORY_KIB,
            DEFAULT_ARGON2_ITERATIONS,
            DEFAULT_ARGON2_PARALLELISM,
        )
        .expect("KDF 派生应当成功");
        let is_invalid = CryptoService::verify_canary(&wrong_key, &canary);
        assert!(!is_invalid, "错误 MasterKey 应当验证失败");
    }

    #[test]
    fn test_aes_gcm_encrypt_decrypt() {
        let key = CryptoService::generate_dek();
        let secret = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIG5... root@server";

        let enc = CryptoService::encrypt(&key, 1, secret.as_bytes()).expect("加密应成功");
        assert!(enc.starts_with("enc:v1:"));

        let (ver, dec_bytes) = CryptoService::decrypt(&key, &enc).expect("解密应成功");
        assert_eq!(ver, 1);
        assert_eq!(String::from_utf8(dec_bytes).unwrap(), secret);
    }
}
