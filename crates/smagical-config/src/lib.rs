//! 应用配置的默认值。
//!
//! 这个模块只负责给启动流程提供稳定的基础配置，并提供全局配置与主机覆盖配置的合并逻辑。
//! 后续落盘和迁移会放在存储层处理，避免 UI 和连接逻辑直接依赖文件格式。

mod background;
mod defaults;
mod visual;

use serde::{Deserialize, Serialize};
use smagical_core::{BackgroundProfile, ThemeProfile};

/// 桌面端运行所需的全局配置。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    pub app_name: String,
    pub theme: ThemeProfile,
    pub background: BackgroundProfile,
}

/// 主机最终生效的视觉配置。
///
/// UI、终端渲染和背景轮转只应读取解析后的结果，避免每个调用点重复处理覆盖和边界值。
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedVisualConfig {
    pub theme: ThemeProfile,
    pub background: BackgroundProfile,
}
