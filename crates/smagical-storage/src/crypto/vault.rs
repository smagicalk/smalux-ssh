//! 保险库生命周期与数据加密密钥 (DEK) 内存状态机。

use std::sync::{Arc, RwLock};
use anyhow::{anyhow, Result};

use crate::crypto::CryptoService;

/// 敏感资产加解密核心控制中枢 (VaultManager)
#[derive(Clone)]
pub struct VaultManager {
    /// 当前活跃的数据加密密钥 (内存即用即存，可锁定抹零)
    active_dek: Arc<RwLock<Option<[u8; 32]>>>,
    /// 当前活跃的 DEK 密钥版本号
    dek_version: Arc<RwLock<u32>>,
}

impl Default for VaultManager {
    fn default() -> Self {
        Self::new_uninitialized()
    }
}

impl VaultManager {
    /// 创建未解锁/未初始化的保险库管理器
    pub fn new_uninitialized() -> Self {
        Self {
            active_dek: Arc::new(RwLock::new(None)),
            dek_version: Arc::new(RwLock::new(1)),
        }
    }

    /// 使用指定 DEK 初始化已解锁状态的保险库
    pub fn new_with_dek(dek: [u8; 32], version: u32) -> Self {
        Self {
            active_dek: Arc::new(RwLock::new(Some(dek))),
            dek_version: Arc::new(RwLock::new(version)),
        }
    }

    /// 查询当前保险库是否已解锁并就绪
    pub fn is_unlocked(&self) -> bool {
        self.active_dek.read().unwrap().is_some()
    }

    /// 获取当前生效的数据密钥版本
    pub fn current_version(&self) -> u32 {
        *self.dek_version.read().unwrap()
    }

    /// 锁定保险库，将内存中的数据密钥从 RAM 中安全清除
    pub fn lock(&self) {
        let mut guard = self.active_dek.write().unwrap();
        *guard = None;
    }

    /// 设置并挂载活跃 DEK
    pub fn unlock_with_dek(&self, dek: [u8; 32], version: u32) {
        let mut dek_guard = self.active_dek.write().unwrap();
        *dek_guard = Some(dek);
        let mut ver_guard = self.dek_version.write().unwrap();
        *ver_guard = version;
    }

    /// 获取当前 DEK 的克隆副本 (用于密钥轮换等运维场景)
    pub fn export_current_dek(&self) -> Result<[u8; 32]> {
        self.active_dek
            .read()
            .unwrap()
            .ok_or_else(|| anyhow!("保险库当前处于锁定状态，无法导出 DEK"))
    }

    /// 加密敏感字符串字段 (若明文为空则直接返回空字符串)
    pub fn encrypt_string(&self, plaintext: &str) -> Result<String> {
        if plaintext.is_empty() {
            return Ok(String::new());
        }

        let dek_opt = *self.active_dek.read().unwrap();
        let dek = dek_opt.ok_or_else(|| anyhow!("保险库当前未解锁，无法加密敏感资产"))?;
        let version = *self.dek_version.read().unwrap();

        CryptoService::encrypt(&dek, version, plaintext.as_bytes())
    }

    /// 解密敏感字符串字段 (若不是 enc:v 前缀则视为历史明文直接放行，平滑兼容)
    pub fn decrypt_string(&self, cipher_or_plain: &str) -> Result<String> {
        if cipher_or_plain.is_empty() {
            return Ok(String::new());
        }

        // 非 enc:v 开头，视为未加密明文，平滑兼容历史数据
        if !cipher_or_plain.starts_with("enc:v") {
            return Ok(cipher_or_plain.to_string());
        }

        let dek_opt = *self.active_dek.read().unwrap();
        let dek = dek_opt.ok_or_else(|| anyhow!("保险库当前未解锁，无法解密敏感资产"))?;

        let (_version, bytes) = CryptoService::decrypt(&dek, cipher_or_plain)?;
        String::from_utf8(bytes).map_err(|e| anyhow!("解密明文包含非法 UTF-8: {}", e))
    }
}
