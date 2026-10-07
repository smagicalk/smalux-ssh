//! plugin-credentials: SSH 密钥、口令密码保管箱与公钥指纹解析插件。

use anyhow::Result;
use smagical_core::CoreState;
use smagical_ui_common::{SidebarPlugin, SidebarPosition};

/// 凭据保管箱插件定义
#[derive(Debug, Default, Clone, Copy)]
pub struct CredentialsPlugin;

impl CredentialsPlugin {
    /// 创建凭据保管箱插件实例
    pub fn new() -> Self {
        Self
    }
}

impl SidebarPlugin for CredentialsPlugin {
    fn id(&self) -> &'static str {
        "credentials"
    }

    fn title(&self, is_en: bool) -> String {
        if is_en { "Credentials" } else { "凭据保管" }.to_string()
    }

    fn icon_name(&self) -> &'static str {
        "key"
    }

    fn position(&self) -> SidebarPosition {
        SidebarPosition::LeftTop
    }

    fn on_register(&self, _core: &CoreState) -> Result<()> {
        Ok(())
    }
}
