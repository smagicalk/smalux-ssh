//! plugin-debug: 开发者调试平台、状态探针与批量数据模拟插件。

use anyhow::Result;
use smagical_core::CoreState;
use smagical_ui_common::{SidebarPlugin, SidebarPosition};

/// 开发者调试插件定义
#[derive(Debug, Default, Clone, Copy)]
pub struct DebugPlugin;

impl DebugPlugin {
    /// 创建开发者调试插件实例
    pub fn new() -> Self {
        Self
    }
}

impl SidebarPlugin for DebugPlugin {
    fn id(&self) -> &'static str {
        "debug"
    }

    fn title(&self, is_en: bool) -> String {
        if is_en { "Debug" } else { "开发调试" }.to_string()
    }

    fn icon_name(&self) -> &'static str {
        "bug"
    }

    fn position(&self) -> SidebarPosition {
        SidebarPosition::LeftBottom
    }

    fn on_register(&self, _core: &CoreState) -> Result<()> {
        Ok(())
    }
}
