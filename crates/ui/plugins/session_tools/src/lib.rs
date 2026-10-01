//! plugin-session-tools: 终端会话伴生工具集（快捷命令、SFTP、Tmux、系统监控探针）。

use anyhow::Result;
use smagical_core::CoreState;
use smagical_ui_common::{SidebarPlugin, SidebarPosition};

slint::include_modules!();

/// 终端伴生工具集插件定义
#[derive(Debug, Default, Clone, Copy)]
pub struct SessionToolsPlugin;

impl SessionToolsPlugin {
    /// 创建会话伴生工具插件实例
    pub fn new() -> Self {
        Self
    }
}

impl SidebarPlugin for SessionToolsPlugin {
    fn id(&self) -> &'static str {
        "session_tools"
    }

    fn title(&self, is_en: bool) -> String {
        if is_en { "Tools" } else { "辅助工具" }.to_string()
    }

    fn icon_name(&self) -> &'static str {
        "tool"
    }

    fn position(&self) -> SidebarPosition {
        SidebarPosition::RightPanel
    }

    fn on_register(&self, _core: &CoreState) -> Result<()> {
        Ok(())
    }
}
