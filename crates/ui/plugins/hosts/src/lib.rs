//! plugin-hosts: 主机资产、树状分组与凭据管理插件。

use anyhow::Result;
use smagical_core::CoreState;
use smagical_ui_common::{SidebarPlugin, SidebarPosition};

/// 主机资产管理插件定义
#[derive(Debug, Default, Clone, Copy)]
pub struct HostsPlugin;

impl HostsPlugin {
    /// 创建主机资产插件实例
    pub fn new() -> Self {
        Self
    }
}

impl SidebarPlugin for HostsPlugin {
    fn id(&self) -> &'static str {
        "hosts"
    }

    fn title(&self, is_en: bool) -> String {
        if is_en { "Hosts" } else { "主机资产" }.to_string()
    }

    fn icon_name(&self) -> &'static str {
        "server"
    }

    fn position(&self) -> SidebarPosition {
        SidebarPosition::LeftTop
    }

    fn on_register(&self, _core: &CoreState) -> Result<()> {
        Ok(())
    }
}
