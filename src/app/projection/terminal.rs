//! Slint 终端面板写入。
//!
//! 终端面板支持局部刷新，所以这里独立于整窗 projection。输入、输出行、连接状态和按钮
//! 可用性全部来自 `TerminalViewModel`，不在 Slint 侧重新判断。

use crate::app::AppWindow;
use crate::app::projection::models::string_model;
use crate::app::view_model::TerminalViewModel;

pub(super) fn sync_terminal_model(window: &AppWindow, model: &TerminalViewModel) {
    // active_session_id 是终端输入回调继续定位核心 session 的桥接字段。
    window.set_active_session_id(model.session_id.as_str().into());
    window.set_active_tab_title(model.title.as_str().into());
    window.set_active_tab_kind(model.kind.into());
    window.set_active_tab_status(model.status.into());
    window.set_terminal_input(model.input.as_str().into());
    window.set_terminal_prompt(model.prompt.into());
    // 输出行保持为字符串列表，终端渲染细节留在 Slint 组件层。
    window.set_terminal_output(string_model(&model.output_lines));
    window.set_terminal_can_send_input(model.can_send_input);
    window.set_terminal_can_reconnect_shell(model.can_reconnect_shell);
}
