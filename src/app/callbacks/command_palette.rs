//! 命令面板回调。

use std::rc::Rc;

use slint::ComponentHandle;

use crate::model::Message;

use super::{
    AppWindow, SharedAppState, apply_and_sync, apply_messages_and_sync, parse_command_history_id,
    parse_host_id,
};

pub(super) fn bind(window: &AppWindow, state: SharedAppState) {
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_open_command_palette(move || {
            apply_and_sync(
                &weak,
                &state,
                Message::OpenCommandPalette {
                    query: String::new(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_update_command_palette(move |query| {
            apply_and_sync(
                &weak,
                &state,
                Message::UpdateCommandPaletteQuery {
                    query: query.to_string(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_close_command_palette(move || {
            apply_and_sync(&weak, &state, Message::CloseCommandPalette);
        });
    }
    {
        let weak = window.as_weak();
        window.on_activate_command_palette_item(move |item_id, kind| {
            let message = match kind.as_str() {
                "Host" | "Recent" => {
                    parse_host_id(&item_id).map(|host_id| Message::OpenShell { host_id })
                }
                "History" => parse_command_history_id(&item_id)
                    .map(|history_id| Message::RunCommandHistory { history_id }),
                _ => None,
            };

            if let Some(message) = message {
                apply_messages_and_sync(&weak, &state, [message, Message::CloseCommandPalette]);
            }
        });
    }
}
