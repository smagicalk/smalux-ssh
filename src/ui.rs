//! 主界面视图组合。
//!
//! 当前页面是工程骨架的可运行仪表盘，用来验证状态、主题切换和后续模块接入点。
//! 后续终端、SFTP、隧道和设置页会继续拆成更小的视图模块。

use iced::{
    Element,
    widget::{button, column, text},
};

use crate::model::{AppState, Message};

/// 根据应用状态构建当前主界面。
pub fn view(state: &AppState) -> Element<'_, Message> {
    let visual = state.config.resolve_visual_for_host(None);

    column![
        text(&state.config.app_name),
        text(format!("hosts: {}", state.storage.host_count())),
        text(format!("groups: {}", state.storage.group_count())),
        text(format!("credentials: {}", state.storage.credential_count())),
        text(format!("known hosts: {}", state.storage.known_host_count())),
        text(format!(
            "workspace tabs: {}",
            state.storage.workspace_tab_count()
        )),
        text(format!("recent: {}", state.storage.recent_count())),
        text(format!(
            "history: {}",
            state.storage.command_history_count()
        )),
        text(format!("snippets: {}", state.storage.snippet_count())),
        text(format!(
            "sftp bookmarks: {}",
            state.storage.sftp_bookmark_count()
        )),
        text(format!(
            "sftp browsers: {}",
            state.sessions.sftp_browser_count()
        )),
        text(format!("transfers: {}", state.sessions.transfer_count())),
        text(format!("tunnels: {}", state.storage.tunnel_rule_count())),
        text(format!(
            "tunnel runtime: {}",
            state.sessions.tunnel_runtime_count()
        )),
        text(format!("sessions: {}", state.sessions.active_count())),
        text(format!("session tabs: {}", state.sessions.tab_count())),
        text(format!("terminal tabs: {}", state.terminal.tab_count())),
        text(format!(
            "local shells: {}",
            state.terminal.local_shell_count()
        )),
        text(format!("theme: {}", visual.theme.name)),
        text(format!(
            "background: {}",
            if visual.background.enabled {
                "enabled"
            } else {
                "disabled"
            }
        )),
        button("Toggle theme").on_press(Message::ToggleTheme),
    ]
    .spacing(12)
    .padding(16)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_accepts_default_state() {
        let state = AppState::default();

        let _element = view(&state);
    }
}
