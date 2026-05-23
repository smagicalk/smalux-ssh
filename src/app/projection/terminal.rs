//! Slint 终端面板写入。

use crate::app::AppWindow;
use crate::app::projection::models::string_model;
use crate::app::view_model::TerminalViewModel;

pub(super) fn sync_terminal_model(window: &AppWindow, model: &TerminalViewModel) {
    window.set_active_session_id(model.session_id.as_str().into());
    window.set_active_tab_title(model.title.as_str().into());
    window.set_active_tab_kind(model.kind.into());
    window.set_active_tab_status(model.status.into());
    window.set_terminal_input(model.input.as_str().into());
    window.set_terminal_prompt(model.prompt.into());
    window.set_terminal_output(string_model(&model.output_lines));
    window.set_terminal_can_send_input(model.can_send_input);
}
