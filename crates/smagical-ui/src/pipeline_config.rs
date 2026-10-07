//! 图形渲染管线本地持久化配置与启动分发模块。
//!
//! 负责在应用启动前自磁盘读取用户配置的渲染管线首选项并注入 `SLINT_BACKEND` 环境变量，
//! 以及在用户更改渲染管线时完成物理文件落盘，实现跨进程与多次启动的真正持久化。
//! 底层全面收拢至 `smagical_core::bootstrap::BootstrapConfig` (`bootstrap.toml`)。

use std::path::PathBuf;
pub use smagical_core::bootstrap::{
    get_bootstrap_config_path, BootstrapConfig, DEFAULT_PIPELINE, VALID_PIPELINES,
};

/// 获取持久化配置文件路径：统一映射至标准 `bootstrap.toml`
pub fn get_pipeline_config_path() -> Option<PathBuf> {
    get_bootstrap_config_path()
}

/// 从物理磁盘读取已持久化的渲染管线首选项
pub fn get_persisted_pipeline() -> Option<String> {
    Some(BootstrapConfig::load().render.pipeline)
}

/// 将渲染管线配置安全写入物理磁盘 (更新 bootstrap.toml 并同步 SLINT_BACKEND 环境变量)
pub fn save_persisted_pipeline(pipeline: &str) -> std::io::Result<()> {
    let trimmed = pipeline.trim();
    if !VALID_PIPELINES.contains(&trimmed) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("非法的渲染管线名称: {}", trimmed),
        ));
    }

    BootstrapConfig::update(|cfg| {
        cfg.render.pipeline = trimmed.to_string();
    })?;

    // 同步设置当前进程环境变量
    unsafe {
        std::env::set_var("SLINT_BACKEND", trimmed);
    }

    tracing::info!(target: "smagical_ui::render", "渲染管线已持久化至引导配置: [{}]", trimmed);
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

    // 优先级 2: 从物理磁盘 bootstrap.toml 读取上一次保存的渲染管线
    let config = BootstrapConfig::load();
    let saved = config.render.pipeline;
    unsafe {
        std::env::set_var("SLINT_BACKEND", &saved);
    }

    // 官方性能监控探针支持 (SLINT_DEBUG_PERFORMANCE=overlay 或 console)
    if let Ok(perf) = std::env::var("SLINT_DEBUG_PERFORMANCE") {
        tracing::info!(
            target: "smalux::render",
            "已激活 Slint 原生图形渲染性能监控 (SLINT_DEBUG_PERFORMANCE={})",
            perf
        );
    }

    saved
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
