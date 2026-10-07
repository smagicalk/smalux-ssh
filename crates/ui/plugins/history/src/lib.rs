//! plugin-history: 终端会话执行审计、会话回放快照与历史连接管理插件。

use anyhow::Result;
use smagical_core::CoreState;
use smagical_ui_common::{SidebarPlugin, SidebarPosition};

/// 终端会话审计与历史管理插件定义
#[derive(Debug, Default, Clone, Copy)]
pub struct HistoryPlugin;

impl HistoryPlugin {
    /// 创建历史审计插件实例
    pub fn new() -> Self {
        Self
    }
}

impl SidebarPlugin for HistoryPlugin {
    fn id(&self) -> &'static str {
        "history"
    }

    fn title(&self, is_en: bool) -> String {
        if is_en { "History" } else { "历史审计" }.to_string()
    }

    fn icon_name(&self) -> &'static str {
        "clock"
    }

    fn position(&self) -> SidebarPosition {
        SidebarPosition::LeftTop
    }

    fn on_register(&self, _core: &CoreState) -> Result<()> {
        Ok(())
    }
}
