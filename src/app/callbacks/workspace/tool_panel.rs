use crate::app::callbacks::{
    AppWindow, SharedAppState, active_terminal_host_id, apply_and_sync, apply_messages_and_sync,
};
use crate::model::{Message, ToolPanelMode};

pub(super) fn open_tool_panel(weak: &slint::Weak<AppWindow>, state: &SharedAppState, mode: &str) {
    let Some(mode) = super::super::parse_tool_panel_mode(mode) else {
        return;
    };
    let host_without_sftp = host_without_sftp_browser(state, mode);

    if let Some(host_id) = host_without_sftp {
        apply_messages_and_sync(
            weak,
            state,
            [
                Message::OpenSftp {
                    host_id,
                    initial_dir: "/".to_owned(),
                },
                Message::OpenToolPanel { mode },
            ],
        );
    } else {
        apply_and_sync(weak, state, Message::OpenToolPanel { mode });
    }
}

fn host_without_sftp_browser(
    state: &SharedAppState,
    mode: ToolPanelMode,
) -> Option<crate::model::HostId> {
    if !matches!(mode, ToolPanelMode::Sftp) {
        return None;
    }

    let state = state.borrow();
    active_terminal_host_id(&state).filter(|host_id| {
        !state
            .sessions
            .sftp_browsers
            .iter()
            .any(|browser| browser.host_id == *host_id)
    })
}
