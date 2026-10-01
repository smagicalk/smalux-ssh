//! 存储后端模式本地持久化配置模块 (Storage Mode Config)。
//!
//! 负责跨启动持久化记录用户选择的存储模式 (物理持久化 SQLite vs 内存种子 Mock)，
//! 可由 UI 界面、CLI 命令行工具或后台守护服务直接共享读取。

use std::fs;
use std::path::PathBuf;

/// 存储模式配置文件路径：~/.config/smalux-ssh/storage_mode.txt
pub fn get_storage_config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("dev", "smagical", "smalux-ssh")
        .map(|dirs| dirs.config_dir().join("storage_mode.txt"))
}

/// 从物理磁盘读取已持久化的存储模式首选项 ("physical" | "mock")
pub fn get_persisted_storage_mode() -> Option<String> {
    let path = get_storage_config_path()?;
    if !path.exists() {
        return None;
    }
    match fs::read_to_string(&path) {
        Ok(content) => {
            let trimmed = content.trim().to_lowercase();
            if trimmed == "mock" || trimmed == "physical" {
                Some(trimmed)
            } else {
                None
            }
        }
        Err(err) => {
            tracing::warn!(target: "smagical_storage::config", "读取存储模式配置文件失败: {:?}", err);
            None
        }
    }
}

/// 将用户选择的存储模式首选项写入物理磁盘
pub fn save_persisted_storage_mode(mode: &str) -> std::io::Result<()> {
    let trimmed = mode.trim().to_lowercase();
    if trimmed != "mock" && trimmed != "physical" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("非法的存储模式: {}", trimmed),
        ));
    }

    if let Some(path) = get_storage_config_path() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, trimmed)?;
    }
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
        assert!(p.to_string_lossy().contains("storage_mode.txt"));
    }
}
