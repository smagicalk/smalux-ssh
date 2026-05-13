//! 主界面视图组合。
//!
//! 顶层只装配页面骨架，具体主机操作和会话概览拆到独立视图模块。

use iced::{
    Element,
    widget::{button, column, row, scrollable, text},
};

use crate::model::{AppState, Message};

mod host_actions;
mod security;
mod session_summary;
mod sftp_workspace;
mod terminal_workspace;
mod visual_settings;
mod workspace;

/// 根据应用状态构建当前主界面。
pub fn view(state: &AppState) -> Element<'_, Message> {
    let visual = state.config.resolve_visual_for_host(None);

    let content = column![
        header(state),
        visual_settings::view(state),
        overview(state, visual.theme.name.clone(), visual.background.enabled),
        workspace::view(state),
        security::view(state),
        host_actions::view(state),
        terminal_workspace::view(state),
        sftp_workspace::view(state),
        session_summary::view(state),
    ]
    .spacing(16)
    .padding(16);

    scrollable(content).into()
}

fn header(state: &AppState) -> Element<'_, Message> {
    row![
        text(&state.config.app_name).size(28),
        button("Toggle theme").on_press(Message::ToggleTheme),
    ]
    .spacing(12)
    .into()
}

fn overview(
    state: &AppState,
    theme_name: String,
    background_enabled: bool,
) -> Element<'_, Message> {
    let background_state = if background_enabled {
        "enabled"
    } else {
        "disabled"
    };

    column![
        text("Overview").size(22),
        text(format!(
            "hosts: {} | groups: {} | recent: {} | history: {}",
            state.storage.host_count(),
            state.storage.group_count(),
            state.storage.recent_count(),
            state.storage.command_history_count()
        )),
        text(format!(
            "credentials: {} | known hosts: {} | snippets: {} | sftp bookmarks: {}",
            state.storage.credential_count(),
            state.storage.known_host_count(),
            state.storage.snippet_count(),
            state.storage.sftp_bookmark_count()
        )),
        text(format!(
            "session tabs: {} | terminal tabs: {} | sftp browsers: {} | transfers: {}",
            state.sessions.tab_count(),
            state.terminal.tab_count(),
            state.sessions.sftp_browser_count(),
            state.sessions.transfer_count()
        )),
        text(format!(
            "tunnel rules: {} | tunnel runtime: {} | backend queue: {} | local shells: {}",
            state.storage.tunnel_rule_count(),
            state.sessions.tunnel_runtime_count(),
            state.backend_commands.pending_count(),
            state.terminal.local_shell_count()
        )),
        text(format!(
            "workspace tabs: {} | theme: {} | background: {}",
            state.storage.workspace_tab_count(),
            theme_name,
            background_state
        )),
    ]
    .spacing(6)
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

    #[test]
    fn view_accepts_state_with_host_and_tunnel_rule() {
        use crate::model::{AuthProfile, Host, HostId, SecretRef, TunnelKind, TunnelRule};

        let mut state = AppState::default();
        let host_id = HostId(uuid::Uuid::new_v4());
        state.storage.upsert_host(Host {
            id: host_id,
            name: "production".to_owned(),
            group_id: None,
            tags: vec!["prod".to_owned()],
            address: "example.com".to_owned(),
            port: 22,
            auth: AuthProfile::Password {
                username: "ops".to_owned(),
                secret: SecretRef("password:ops".to_owned()),
            },
            proxy: None,
            jumps: Vec::new(),
            theme_override: None,
            background_override: None,
        });
        state.storage.upsert_tunnel_rule(TunnelRule {
            name: "local-db".to_owned(),
            kind: TunnelKind::Local,
            bind_host: "127.0.0.1".to_owned(),
            bind_port: 15432,
            target_host: "10.0.0.5".to_owned(),
            target_port: 5432,
            auto_start: false,
        });

        let _element = view(&state);
    }
}
