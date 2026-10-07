//! plugin-files: 双盘 SFTP 文件管理器、传输队列与远程文件浏览插件。

use anyhow::Result;
use smagical_core::CoreState;
use smagical_ui_common::{SidebarPlugin, SidebarPosition};

/// 双盘文件管理与 SFTP 传输插件定义
#[derive(Debug, Default, Clone, Copy)]
pub struct FilesPlugin;

impl FilesPlugin {
    /// 创建文件管理插件实例
    pub fn new() -> Self {
        Self
    }
}

impl SidebarPlugin for FilesPlugin {
    fn id(&self) -> &'static str {
        "files"
    }

    fn title(&self, is_en: bool) -> String {
        if is_en { "Files" } else { "文件管理" }.to_string()
    }

    fn icon_name(&self) -> &'static str {
        "folder"
    }

    fn position(&self) -> SidebarPosition {
        SidebarPosition::LeftTop
    }

    fn on_register(&self, _core: &CoreState) -> Result<()> {
        Ok(())
    }
}
