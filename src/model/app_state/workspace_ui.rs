//! 工作区页面、分栏和命令面板的 UI 消息处理。

#[path = "workspace_ui/background.rs"]
mod background;
#[path = "workspace_ui/command_palette.rs"]
mod command_palette;
#[path = "workspace_ui/layout.rs"]
mod layout;
#[path = "workspace_ui/page.rs"]
mod page;
#[cfg(test)]
#[path = "workspace_ui/tests.rs"]
mod tests;
#[path = "workspace_ui/tool_panel.rs"]
mod tool_panel;
