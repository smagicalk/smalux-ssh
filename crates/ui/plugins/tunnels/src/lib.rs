//! plugin-tunnels: SSH 动态端口转发、网络隧道拓扑与代理跳板插件。

use anyhow::Result;
use smagical_core::CoreState;
use smagical_ui_common::{SidebarPlugin, SidebarPosition};

/// 网络隧道插件定义
#[derive(Debug, Default, Clone, Copy)]
pub struct TunnelsPlugin;

impl TunnelsPlugin {
    /// 创建网络隧道插件实例
    pub fn new() -> Self {
        Self
    }
}

impl SidebarPlugin for TunnelsPlugin {
    fn id(&self) -> &'static str {
        "tunnels"
    }

    fn title(&self, is_en: bool) -> String {
        if is_en { "Tunnels" } else { "网络隧道" }.to_string()
    }

    fn icon_name(&self) -> &'static str {
        "git-commit"
    }

    fn position(&self) -> SidebarPosition {
        SidebarPosition::LeftTop
    }

    fn on_register(&self, _core: &CoreState) -> Result<()> {
        Ok(())
    }
}
