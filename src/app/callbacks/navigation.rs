//! 工作区页面导航回调。
//!
//! 导航回调是最薄的一类 Adapter：每个 Slint 点击事件只映射到一个
//! `WorkspacePage`，实际的当前页面状态由核心 `AppState` 保存。

use std::rc::Rc;

use slint::ComponentHandle;

use crate::model::{Message, WorkspacePage};

use super::{AppWindow, SharedAppState, apply_and_sync};

pub(super) fn bind(window: &AppWindow, state: SharedAppState) {
    // 所有页面都走同一个 bind_page，避免每个按钮复制一段闭包样板。
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
        Rc::clone(&state),
        WindowCallback::Security,
        WorkspacePage::Security,
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
        // 导航只是状态切换，不直接操作 Slint 可见性；可见性由 projection 统一写回。
        apply_and_sync(&weak, &state, Message::NavigateWorkspacePage { page });
    };

    match callback {
        WindowCallback::Hosts => window.on_open_hosts(handler),
        WindowCallback::Terminal => window.on_open_terminal(handler),
        WindowCallback::Sftp => window.on_open_sftp(handler),
        WindowCallback::Tunnels => window.on_open_tunnels(handler),
        WindowCallback::Snippets => window.on_open_snippets(handler),
        WindowCallback::History => window.on_open_history(handler),
        WindowCallback::Security => window.on_open_security(handler),
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
    Security,
    Settings,
}
