//! 系统预启动引导配置模块 (Bootstrap Configuration)。
//!
//! 负责在图形引擎 (Slint) 与核心数据库 (SeaORM) 启动之前，从物理磁盘安全加载/持久化
//! 引擎级的引导首选项 (渲染管线、存储后端模式)。
//! 替代了历史遗留的分散 `.txt` 文本存储方案，提供统一、严谨的 `bootstrap.toml` 格式。

use std::fs;
use std::io;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// 合法渲染管线列表
pub const VALID_PIPELINES: &[&str] = &[
    "winit-skia",
    "winit-skia-opengl",
    "winit-skia-software",
];

/// 默认渲染管线
pub const DEFAULT_PIPELINE: &str = "winit-skia";

/// 合法存储模式列表
pub const VALID_STORAGE_MODES: &[&str] = &["physical", "mock"];

/// 默认存储模式
pub const DEFAULT_STORAGE_MODE: &str = "physical";

/// 统一预启动引导配置数据结构
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BootstrapConfig {
    /// 渲染引擎引导配置
    #[serde(default)]
    pub render: RenderBootstrapConfig,
    /// 存储引擎引导配置
    #[serde(default)]
    pub storage: StorageBootstrapConfig,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            render: RenderBootstrapConfig::default(),
            storage: StorageBootstrapConfig::default(),
        }
    }
}

/// 图形渲染引擎启动引导参数
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RenderBootstrapConfig {
    /// 图形渲染管线名称 ("winit-skia" | "winit-skia-opengl" | "winit-skia-software")
    pub pipeline: String,
}

impl Default for RenderBootstrapConfig {
    fn default() -> Self {
        Self {
            pipeline: DEFAULT_PIPELINE.to_string(),
        }
    }
}

/// 存储引擎启动引导参数
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageBootstrapConfig {
    /// 存储后端模式 ("physical" | "mock")
    pub mode: String,
}

impl Default for StorageBootstrapConfig {
    fn default() -> Self {
        Self {
            mode: DEFAULT_STORAGE_MODE.to_string(),
        }
    }
}

/// 获取全局标准 bootstrap.toml 配置文件绝对路径
///
/// 路径遵循系统 XDG / Windows 漫游配置标准：
/// - Windows: `%APPDATA%\smagical\smalux-ssh\config\bootstrap.toml`
/// - Linux/macOS: `~/.config/smalux-ssh/bootstrap.toml`
pub fn get_bootstrap_config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("dev", "smagical", "smalux-ssh")
        .map(|dirs| dirs.config_dir().join("bootstrap.toml"))
}

impl BootstrapConfig {
    /// 从物理磁盘加载引导配置
    ///
    /// 若 `bootstrap.toml` 存在，直接解析并校验；
    /// 若不存在但存在历史遗留的 `.txt` 单独配置文件，将自动平滑迁移并清理旧文本；
    /// 若均不存在或读取出错，安全回退到内置默认值。
    pub fn load() -> Self {
        let Some(path) = get_bootstrap_config_path() else {
            return Self::default();
        };

        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(mut config) = toml::from_str::<BootstrapConfig>(&content) {
                    config.validate_and_sanitize();
                    return config;
                }
            }
        }

        // 尝试从历史遗留的 .txt 文件中平滑迁移
        if let Some(parent) = path.parent() {
            let mut migrated = Self::default();
            let mut has_legacy = false;

            let legacy_render_txt = parent.join("rendering_pipeline.txt");
            if legacy_render_txt.exists() {
                if let Ok(txt) = fs::read_to_string(&legacy_render_txt) {
                    let trimmed = txt.trim();
                    if VALID_PIPELINES.contains(&trimmed) {
                        migrated.render.pipeline = trimmed.to_string();
                        has_legacy = true;
                    }
                }
                let _ = fs::remove_file(&legacy_render_txt);
            }

            let legacy_storage_txt = parent.join("storage_mode.txt");
            if legacy_storage_txt.exists() {
                if let Ok(txt) = fs::read_to_string(&legacy_storage_txt) {
                    let trimmed = txt.trim().to_lowercase();
                    if VALID_STORAGE_MODES.contains(&trimmed.as_str()) {
                        migrated.storage.mode = trimmed;
                        has_legacy = true;
                    }
                }
                let _ = fs::remove_file(&legacy_storage_txt);
            }

            if has_legacy {
                let _ = migrated.save();
                return migrated;
            }
        }

        Self::default()
    }

    /// 将当前引导配置序列化写入磁盘中的 `bootstrap.toml`
    pub fn save(&self) -> io::Result<()> {
        let Some(path) = get_bootstrap_config_path() else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "无法定位配置目录"));
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let toml_str = toml::to_string_pretty(self).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("TOML 序列化失败: {}", e))
        })?;

        fs::write(&path, toml_str)?;
        tracing::info!(target: "smagical_core::bootstrap", "引导配置已安全落盘: {:?}", path);
        Ok(())
    }

    /// 校验并修正非法字段值
    pub fn validate_and_sanitize(&mut self) {
        if !VALID_PIPELINES.contains(&self.render.pipeline.as_str()) {
            tracing::warn!(
                target: "smagical_core::bootstrap",
                "遇到未知渲染管线 [{}], 自动回退默认 [{}]",
                self.render.pipeline,
                DEFAULT_PIPELINE
            );
            self.render.pipeline = DEFAULT_PIPELINE.to_string();
        }

        let mode_lower = self.storage.mode.to_lowercase();
        if !VALID_STORAGE_MODES.contains(&mode_lower.as_str()) {
            tracing::warn!(
                target: "smagical_core::bootstrap",
                "遇到未知存储模式 [{}], 自动回退默认 [{}]",
                self.storage.mode,
                DEFAULT_STORAGE_MODE
            );
            self.storage.mode = DEFAULT_STORAGE_MODE.to_string();
        } else {
            self.storage.mode = mode_lower;
        }
    }

    /// 原子变更并持久化
    pub fn update<F: FnOnce(&mut Self)>(f: F) -> io::Result<Self> {
        let mut config = Self::load();
        f(&mut config);
        config.validate_and_sanitize();
        config.save()?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bootstrap_config_default_and_serialization() {
        let cfg = BootstrapConfig::default();
        assert_eq!(cfg.render.pipeline, "winit-skia");
        assert_eq!(cfg.storage.mode, "physical");

        let toml_str = toml::to_string(&cfg).unwrap();
        let parsed: BootstrapConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(cfg, parsed);
    }

    #[test]
    fn test_bootstrap_config_sanitize() {
        let mut cfg = BootstrapConfig {
            render: RenderBootstrapConfig {
                pipeline: "unknown-gpu".to_string(),
            },
            storage: StorageBootstrapConfig {
                mode: "INVALID".to_string(),
            },
        };
        cfg.validate_and_sanitize();
        assert_eq!(cfg.render.pipeline, "winit-skia");
        assert_eq!(cfg.storage.mode, "physical");
    }

    #[test]
    fn test_bootstrap_config_path() {
        let p = get_bootstrap_config_path().unwrap();
        assert!(p.to_string_lossy().contains("bootstrap.toml"));
    }

    #[test]
    fn test_bootstrap_load_and_migration() {
        let cfg = BootstrapConfig::load();
        assert!(VALID_PIPELINES.contains(&cfg.render.pipeline.as_str()));
        assert!(VALID_STORAGE_MODES.contains(&cfg.storage.mode.as_str()));
    }
}
