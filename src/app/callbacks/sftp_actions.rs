//! SFTP 回调。

use std::rc::Rc;

use slint::ComponentHandle;

use crate::model::Message;

use super::{AppWindow, SharedAppState, apply_and_sync, parse_host_id};

pub(super) fn bind(window: &AppWindow, state: SharedAppState) {
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_refresh_sftp(move |host_id| {
            let Some(host_id) = parse_host_id(&host_id) else {
                return;
            };
            apply_and_sync(&weak, &state, Message::RefreshSftp { host_id });
        });
    }
    {
        let weak = window.as_weak();
        window.on_select_sftp_entry(move |host_id, path| {
            let Some(host_id) = parse_host_id(&host_id) else {
                return;
            };
            apply_and_sync(
                &weak,
                &state,
                Message::SelectSftpEntry {
                    host_id,
                    remote_path: path.to_string(),
                },
            );
        });
    }
}
