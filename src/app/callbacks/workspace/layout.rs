use std::rc::Rc;

use slint::ComponentHandle;

use crate::app::callbacks::{AppWindow, SharedAppState, apply_and_sync};
use crate::model::Message;

pub(super) fn bind(window: &AppWindow, state: &SharedAppState) {
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_toggle_host_list_mode(move || {
            apply_and_sync(&weak, &state, Message::ToggleHostListMode);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_update_host_search(move |query| {
            apply_and_sync(
                &weak,
                &state,
                Message::UpdateHostSearchQuery {
                    query: query.to_string(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_toggle_right_sidebar(move || {
            apply_and_sync(&weak, &state, Message::ToggleRightSidebar);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_resize_hosts_panel(move |width| {
            apply_and_sync(&weak, &state, Message::ResizeHostsPanel { width });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_resize_activity_panel(move |width| {
            apply_and_sync(&weak, &state, Message::ResizeActivityPanel { width });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_resize_tool_panel(move |width| {
            apply_and_sync(&weak, &state, Message::ResizeToolPanel { width });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_next_background(move || {
            apply_and_sync(&weak, &state, Message::NextBackground);
        });
    }
}
