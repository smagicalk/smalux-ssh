//! 图形渲染管线本地持久化配置与启动分发模块。
//!
//! 负责在应用启动前自磁盘读取用户配置的渲染管线首选项并注入 `SLINT_BACKEND` 环境变量，
//! 以及在用户更改渲染管线时完成物理文件落盘，实现跨进程与多次启动的真正持久化。

use std::fs;
use std::path::PathBuf;

/// 默认合法渲染管线列表
pub const VALID_PIPELINES: &[&str] = &[
    "winit-skia",
    "winit-skia-opengl",
    "winit-skia-software",
];

/// 默认推荐渲染管线
pub const DEFAULT_PIPELINE: &str = "winit-skia";

/// 获取持久化配置文件路径：~/.config/smalux-ssh/rendering_pipeline.txt
pub fn get_pipeline_config_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("dev", "smagical", "smalux-ssh")
        .map(|dirs| dirs.config_dir().join("rendering_pipeline.txt"))
}

/// 从物理磁盘读取已持久化的渲染管线首选项
pub fn get_persisted_pipeline() -> Option<String> {
    let path = get_pipeline_config_path()?;
    if !path.exists() {
        return None;
    }
    match fs::read_to_string(&path) {
        Ok(content) => {
            let trimmed = content.trim();
            if VALID_PIPELINES.contains(&trimmed) {
                Some(trimmed.to_string())
            } else {
                tracing::warn!(target: "smagical_ui::render", "读取到无效渲染管线配置: [{}], 将回退默认", trimmed);
                None
            }
        }
        Err(err) => {
            tracing::warn!(target: "smagical_ui::render", "读取渲染管线配置文件失败: {:?}", err);
            None
        }
    }
}

/// 将渲染管线配置安全写入物理磁盘
pub fn save_persisted_pipeline(pipeline: &str) -> std::io::Result<()> {
    let trimmed = pipeline.trim();
    if !VALID_PIPELINES.contains(&trimmed) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("非法的渲染管线名称: {}", trimmed),
        ));
    }

    if let Some(path) = get_pipeline_config_path() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, trimmed)?;
        tracing::info!(target: "smagical_ui::render", "渲染管线已持久化至物理磁盘: [{}] -> {:?}", trimmed, path);
    }

    // 同步设置当前进程环境变量
    unsafe {
        std::env::set_var("SLINT_BACKEND", trimmed);
    }

    Ok(())
}

/// 初始化运行时渲染管线环境变量 (在 Slint 创建 Window 之前调用)
pub fn init_runtime_pipeline() -> String {
    // 优先级 1: 如果当前进程已有环境变量 SLINT_BACKEND 且合法，则优先使用并保存
    if let Ok(env_pipe) = std::env::var("SLINT_BACKEND") {
        let trimmed = env_pipe.trim();
        if VALID_PIPELINES.contains(&trimmed) {
            let _ = save_persisted_pipeline(trimmed);
            return trimmed.to_string();
        }
    }

    // 优先级 2: 从物理磁盘读取上一次保存的渲染管线
    if let Some(saved) = get_persisted_pipeline() {
        unsafe {
            std::env::set_var("SLINT_BACKEND", &saved);
        }
        return saved;
    }

    // 优先级 3: 缺省默认管线
    unsafe {
        std::env::set_var("SLINT_BACKEND", DEFAULT_PIPELINE);
    }
    DEFAULT_PIPELINE.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_pipelines_contain_expected() {
        assert!(VALID_PIPELINES.contains(&"winit-skia"));
        assert!(VALID_PIPELINES.contains(&"winit-skia-opengl"));
        assert!(VALID_PIPELINES.contains(&"winit-skia-software"));
        assert_eq!(VALID_PIPELINES.len(), 3);
    }

    #[test]
    fn test_invalid_pipeline_rejected() {
        assert!(save_persisted_pipeline("invalid-renderer").is_err());
        assert!(save_persisted_pipeline("winit-femtovg").is_err());
    }
}

