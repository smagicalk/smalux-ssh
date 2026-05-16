//! 工作区页面导航回调。

use std::rc::Rc;

use slint::ComponentHandle;

use crate::model::{Message, WorkspacePage};

use super::{AppWindow, SharedAppState, apply_and_sync};

pub(super) fn bind(window: &AppWindow, state: SharedAppState) {
    bind_page(
        window,
        Rc::clone(&state),
        WindowCallback::Hosts,
        WorkspacePage::Hosts,
    );
    bind_page(
        window,
        Rc::clone(&state),
        WindowCallback::Terminal,
        WorkspacePage::Terminal,
    );
    bind_page(
        window,
        Rc::clone(&state),
        WindowCallback::Sftp,
        WorkspacePage::Sftp,
    );
    bind_page(
        window,
        Rc::clone(&state),
        WindowCallback::Tunnels,
        WorkspacePage::Tunnels,
    );
    bind_page(
        window,
        Rc::clone(&state),
        WindowCallback::Snippets,
        WorkspacePage::Snippets,
    );
    bind_page(
        window,
        Rc::clone(&state),
        WindowCallback::History,
        WorkspacePage::History,
    );
    bind_page(
        window,
        state,
        WindowCallback::Settings,
        WorkspacePage::Settings,
    );
}

fn bind_page(
    window: &AppWindow,
    state: SharedAppState,
    callback: WindowCallback,
    page: WorkspacePage,
) {
    let weak = window.as_weak();
    let handler = move || {
        apply_and_sync(&weak, &state, Message::SetWorkspacePage { page });
    };

    match callback {
        WindowCallback::Hosts => window.on_open_hosts(handler),
        WindowCallback::Terminal => window.on_open_terminal(handler),
        WindowCallback::Sftp => window.on_open_sftp(handler),
        WindowCallback::Tunnels => window.on_open_tunnels(handler),
        WindowCallback::Snippets => window.on_open_snippets(handler),
        WindowCallback::History => window.on_open_history(handler),
        WindowCallback::Settings => window.on_open_settings(handler),
    }
}

enum WindowCallback {
    Hosts,
    Terminal,
    Sftp,
    Tunnels,
    Snippets,
    History,
    Settings,
}
