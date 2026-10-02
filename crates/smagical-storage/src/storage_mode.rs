//! 存储后端模式本地持久化配置模块 (Storage Mode Config)。
//!
//! 负责跨启动持久化记录用户选择的存储模式 (物理持久化 SQLite vs 内存种子 Mock)，
//! 可由 UI 界面、CLI 命令行工具或后台守护服务直接共享读取。
//! 底层已全面收拢至 `smagical_core::bootstrap::BootstrapConfig` (`bootstrap.toml`) 规范化管理。

use std::path::PathBuf;
use smagical_core::bootstrap::{get_bootstrap_config_path, BootstrapConfig, VALID_STORAGE_MODES};

/// 存储模式配置文件路径：统一映射至标准 `bootstrap.toml`
pub fn get_storage_config_path() -> Option<PathBuf> {
    get_bootstrap_config_path()
}

/// 从物理磁盘读取已持久化的存储模式首选项 ("physical" | "mock")
pub fn get_persisted_storage_mode() -> Option<String> {
    Some(BootstrapConfig::load().storage.mode)
}

/// 将用户选择的存储模式首选项写入物理磁盘 (更新 bootstrap.toml)
pub fn save_persisted_storage_mode(mode: &str) -> std::io::Result<()> {
    let trimmed = mode.trim().to_lowercase();
    if !VALID_STORAGE_MODES.contains(&trimmed.as_str()) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("非法的存储模式: {}", trimmed),
        ));
    }

    BootstrapConfig::update(|cfg| {
        cfg.storage.mode = trimmed;
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_storage_mode_rejected() {
        let err = save_persisted_storage_mode("invalid_mode");
        assert!(err.is_err());
    }

    #[test]
    fn test_storage_config_path_structure() {
        let path = get_storage_config_path();
        assert!(path.is_some());
        let p = path.unwrap();
        assert!(p.to_string_lossy().contains("bootstrap.toml"));
    }
}
