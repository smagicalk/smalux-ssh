//! plugin-settings: 系统偏好设置、外观主题设计工坊、数据备份与快捷键管理插件。

use anyhow::Result;
use smagical_core::CoreState;
use smagical_ui_common::{SidebarPlugin, SidebarPosition};

/// 偏好设置与主题工坊插件定义
#[derive(Debug, Default, Clone, Copy)]
pub struct SettingsPlugin;

impl SettingsPlugin {
    /// 创建偏好设置插件实例
    pub fn new() -> Self {
        Self
    }
}

impl SidebarPlugin for SettingsPlugin {
    fn id(&self) -> &'static str {
        "settings"
    }

    fn title(&self, is_en: bool) -> String {
        if is_en { "Settings" } else { "偏好设置" }.to_string()
    }

    fn icon_name(&self) -> &'static str {
        "settings"
    }

    fn position(&self) -> SidebarPosition {
        SidebarPosition::LeftBottom
    }

    fn on_register(&self, _core: &CoreState) -> Result<()> {
        Ok(())
    }
}
