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
    #[serde(default)]
    pub workspace: WorkspacePreferences,
    #[serde(default)]
    pub security: SecurityPreferences,
}

/// 可跨启动保留的工作区偏好。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspacePreferences {
    pub host_list_mode: HostListModePreference,
    #[serde(default)]
    pub language: LanguagePreference,
    #[serde(default)]
    pub built_in_theme: BuiltInThemePreference,
}

/// 可跨启动保留的安全偏好。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityPreferences {
    #[serde(default)]
    pub encryption: StorageEncryptionPreference,
}

/// 本地数据库加密偏好。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageEncryptionPreference {
    Disabled,
}

impl Default for StorageEncryptionPreference {
    fn default() -> Self {
        Self::Disabled
    }
}

/// 主机列表展示模式偏好。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostListModePreference {
    Tree,
    Card,
}

/// UI 语言偏好。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LanguagePreference {
    FollowSystem,
    Chinese,
    English,
}

impl Default for LanguagePreference {
    fn default() -> Self {
        Self::FollowSystem
    }
}

/// 内置主题偏好。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuiltInThemePreference {
    ProfessionalDark,
    CatppuccinMocha,
    NordDark,
    Dracula,
    SolarizedDark,
    OceanDark,
    ForestDark,
}

impl Default for BuiltInThemePreference {
    fn default() -> Self {
        Self::ProfessionalDark
    }
}

/// 主机最终生效的视觉配置。
///
/// UI、终端渲染和背景轮转只应读取解析后的结果，避免每个调用点重复处理覆盖和边界值。
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedVisualConfig {
    pub theme: ThemeProfile,
    pub background: BackgroundProfile,
}
