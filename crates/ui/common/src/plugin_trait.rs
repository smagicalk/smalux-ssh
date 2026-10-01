//! 侧边栏插件生命周期契约。
//!
//! 定义微内核架构下所有功能插件（主机资产、代码片段、网络隧道、伴生工具等）的统一接入契约。

use anyhow::Result;
use smagical_core::CoreState;

/// 侧边栏停靠位置
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarPosition {
    /// 左侧主导航栏顶部
    LeftTop,
    /// 左侧主导航栏底部
    LeftBottom,
    /// 右侧辅助工具栏
    RightPanel,
}

/// 侧边栏插件统一抽象接口
pub trait SidebarPlugin: Send + Sync {
    /// 插件唯一标识符 (例如 "hosts", "snippets", "tunnels", "session_tools")
    fn id(&self) -> &'static str;

    /// 插件显示名称 (支持 i18n 国际化)
    fn title(&self, is_en: bool) -> String;

    /// 插件图标名称 (SVG 矢量或 Slint 内置资源名)
    fn icon_name(&self) -> &'static str;

    /// 侧边栏挂载位置
    fn position(&self) -> SidebarPosition;

    /// 插件生命周期：注册钩子，将自身注入核心状态总线
    fn on_register(&self, core: &CoreState) -> Result<()>;
}
