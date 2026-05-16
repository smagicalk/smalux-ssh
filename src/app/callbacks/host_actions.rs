//! 主机动作回调。

use std::rc::Rc;

use slint::ComponentHandle;

use crate::model::{DEFAULT_REMOTE_COMMAND, Message, ToolPanelMode};

use super::{AppWindow, SharedAppState, apply_and_sync, apply_messages_and_sync, parse_host_id};

pub(super) fn bind(window: &AppWindow, state: SharedAppState) {
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_open_shell(move |host_id| {
            let Some(host_id) = parse_host_id(&host_id) else {
                return;
            };
            apply_and_sync(&weak, &state, Message::OpenShell { host_id });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_open_host_sftp(move |host_id| {
            let Some(host_id) = parse_host_id(&host_id) else {
                return;
            };
            apply_messages_and_sync(
                &weak,
                &state,
                [
                    Message::OpenSftp {
                        host_id,
                        initial_dir: "/".to_owned(),
                    },
                    Message::OpenToolPanel {
                        mode: ToolPanelMode::Sftp,
                    },
                ],
            );
        });
    }
    {
        let weak = window.as_weak();
        window.on_run_host_command(move |host_id| {
            let Some(host_id) = parse_host_id(&host_id) else {
                return;
            };
            apply_and_sync(
                &weak,
                &state,
                Message::RunRemoteCommand {
                    host_id,
                    command: DEFAULT_REMOTE_COMMAND.to_owned(),
                    request_pty: true,
                },
            );
        });
    }
}
