//! plugin-snippets: 代码片段树状管理、多语言脚本库与动态参数填报插件。

use anyhow::Result;
use smagical_core::CoreState;
use smagical_ui_common::{SidebarPlugin, SidebarPosition};

/// 代码片段插件定义
#[derive(Debug, Default, Clone, Copy)]
pub struct SnippetsPlugin;

impl SnippetsPlugin {
    /// 创建代码片段插件实例
    pub fn new() -> Self {
        Self
    }
}

impl SidebarPlugin for SnippetsPlugin {
    fn id(&self) -> &'static str {
        "snippets"
    }

    fn title(&self, is_en: bool) -> String {
        if is_en { "Snippets" } else { "代码片段" }.to_string()
    }

    fn icon_name(&self) -> &'static str {
        "code"
    }

    fn position(&self) -> SidebarPosition {
        SidebarPosition::LeftTop
    }

    fn on_register(&self, _core: &CoreState) -> Result<()> {
        Ok(())
    }
}
