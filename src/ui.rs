//! 主界面视图组合。
//!
//! 顶层只装配页面骨架，具体主机操作和会话概览拆到独立视图模块。

use iced::{
    Alignment, Background, Border, Color, Element, Length, Shadow, Theme, Vector,
    widget::{Space, button, column, container, row, rule, scrollable, text},
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

    let mut content = column![top_bar(state)];
    if let Some(error) = state.ui.last_error.as_deref() {
        content = content.push(error_banner(error));
    }

    content = content.push(
        row![
            sidebar(state),
            rule::vertical(1),
            main_workbench(state, visual.theme.name.clone(), visual.background.enabled),
            rule::vertical(1),
            inspector(state),
        ]
        .height(Length::Fill),
    );

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(app_background_style)
        .into()
}

fn top_bar(state: &AppState) -> Element<'_, Message> {
    let status = format!(
        "{} hosts  |  {} tabs  |  {} queued  |  {} snippets",
        state.storage.host_count(),
        state.sessions.tab_count(),
        state.backend_commands.pending_count(),
        state.storage.snippet_count()
    );

    container(
        row![
            column![
                text(&state.config.app_name).size(24).color(TEXT_STRONG),
                text("Cross-platform SSH workspace")
                    .size(12)
                    .color(TEXT_MUTED),
            ]
            .spacing(2),
            Space::new().width(Length::Fill),
            text(status).size(13).color(TEXT_MUTED),
            button("Toggle theme").on_press(Message::ToggleTheme),
        ]
        .spacing(16)
        .align_y(Alignment::Center),
    )
    .padding([14, 18])
    .width(Length::Fill)
    .style(top_bar_style)
    .into()
}

fn sidebar(state: &AppState) -> Element<'_, Message> {
    let active = state.sessions.tab_count();
    let transfer = state.sessions.transfer_count();
    let tunnels = state.sessions.tunnel_runtime_count();

    container(
        column![
            text("Workspace").size(12).color(TEXT_MUTED),
            nav_item("Hosts", state.storage.host_count(), true),
            nav_item("Terminal", state.terminal.tab_count(), false),
            nav_item("SFTP", state.sessions.sftp_browser_count(), false),
            nav_item("Tunnels", tunnels, false),
            nav_item("History", state.storage.command_history_count(), false),
            nav_item("Security", state.storage.credential_count(), false),
            rule::horizontal(1),
            text("Runtime").size(12).color(TEXT_MUTED),
            sidebar_fact("active sessions", active),
            sidebar_fact("transfers", transfer),
            sidebar_fact("recent", state.storage.recent_count()),
            Space::new().height(Length::Fill),
            text("Reference: Termius / Termora / XTerminal")
                .size(11)
                .color(TEXT_MUTED),
        ]
        .spacing(10),
    )
    .padding(16)
    .width(240)
    .height(Length::Fill)
    .style(sidebar_style)
    .into()
}

fn nav_item(label: &'static str, count: usize, selected: bool) -> Element<'static, Message> {
    let marker = if selected { ">" } else { " " };
    let color = if selected { TEXT_STRONG } else { TEXT_SOFT };

    container(
        row![
            text(marker).size(14).color(ACCENT),
            text(label).size(15).color(color).width(Length::Fill),
            text(count.to_string()).size(12).color(TEXT_MUTED),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([8, 10])
    .width(Length::Fill)
    .style(if selected {
        selected_nav_style
    } else {
        transparent_style
    })
    .into()
}

fn sidebar_fact(label: &'static str, value: usize) -> Element<'static, Message> {
    row![
        text(label).size(12).color(TEXT_MUTED).width(Length::Fill),
        text(value.to_string()).size(12).color(TEXT_SOFT),
    ]
    .spacing(8)
    .into()
}

fn error_banner(error: &str) -> Element<'_, Message> {
    container(
        row![
            text(format!("Error: {error}"))
                .size(13)
                .color(Color::from_rgb8(255, 230, 204))
                .width(Length::Fill),
            button("Dismiss").on_press(Message::DismissUiError),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding([10, 18])
    .width(Length::Fill)
    .style(error_banner_style)
    .into()
}

fn main_workbench(
    state: &AppState,
    theme_name: String,
    background_enabled: bool,
) -> Element<'_, Message> {
    container(scrollable(
        column![
            workbench_header(theme_name, background_enabled),
            kpi_row(state),
            panel(
                "Connection Workbench",
                "Saved hosts, SSH shell, SFTP, remote commands, PTY commands and snippets.",
                host_actions::view(state),
            ),
            panel(
                "Terminal",
                "Active shell and remote command output.",
                terminal_workspace::view(state),
            ),
            panel(
                "SFTP",
                "Remote file browser and transfers.",
                sftp_workspace::view(state),
            ),
        ]
        .spacing(14)
        .padding(18),
    ))
    .width(Length::FillPortion(3))
    .height(Length::Fill)
    .style(workbench_style)
    .into()
}

fn workbench_header(theme_name: String, background_enabled: bool) -> Element<'static, Message> {
    let background_state = if background_enabled {
        "background rotation on"
    } else {
        "background rotation off"
    };

    container(
        row![
            column![
                text("Connection Center").size(26).color(TEXT_STRONG),
                text(format!("theme: {theme_name}  |  {background_state}"))
                    .size(13)
                    .color(TEXT_MUTED),
            ]
            .spacing(4),
            Space::new().width(Length::Fill),
            button("Save workspace").on_press(Message::SaveWorkspaceSnapshot),
            button("Restore").on_press(Message::RestoreWorkspaceSnapshot),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .padding(18)
    .width(Length::Fill)
    .style(hero_style)
    .into()
}

fn kpi_row(state: &AppState) -> Element<'_, Message> {
    row![
        metric_card("Hosts", state.storage.host_count(), "saved assets"),
        metric_card("Sessions", state.sessions.tab_count(), "open tabs"),
        metric_card("SFTP", state.sessions.sftp_browser_count(), "browsers"),
        metric_card("Tunnels", state.sessions.tunnel_runtime_count(), "runtime"),
    ]
    .spacing(12)
    .into()
}

fn metric_card(title: &'static str, value: usize, hint: &'static str) -> Element<'static, Message> {
    container(
        column![
            text(title).size(12).color(TEXT_MUTED),
            text(value.to_string()).size(26).color(TEXT_STRONG),
            text(hint).size(11).color(TEXT_MUTED),
        ]
        .spacing(4),
    )
    .padding(14)
    .width(Length::Fill)
    .style(metric_style)
    .into()
}

fn panel<'a>(
    title: &'static str,
    subtitle: &'static str,
    content: Element<'a, Message>,
) -> Element<'a, Message> {
    container(
        column![
            row![
                column![
                    text(title).size(18).color(TEXT_STRONG),
                    text(subtitle).size(12).color(TEXT_MUTED),
                ]
                .spacing(2),
                Space::new().width(Length::Fill),
            ],
            rule::horizontal(1),
            content,
        ]
        .spacing(12),
    )
    .padding(16)
    .width(Length::Fill)
    .style(panel_style)
    .into()
}

fn inspector(state: &AppState) -> Element<'_, Message> {
    container(scrollable(
        column![
            compact_overview(state),
            panel(
                "Sessions",
                "Tabs, transfers, tunnels and recent activity.",
                session_summary::view(state),
            ),
            panel(
                "Workspace",
                "Saved layout snapshot.",
                workspace::view(state),
            ),
            panel(
                "Visuals",
                "Global and host visual settings.",
                visual_settings::view(state),
            ),
            panel(
                "Security",
                "Credentials and known hosts.",
                security::view(state),
            ),
        ]
        .spacing(14)
        .padding(14),
    ))
    .width(360)
    .height(Length::Fill)
    .style(inspector_style)
    .into()
}

fn compact_overview(state: &AppState) -> Element<'_, Message> {
    column![
        text("Overview").size(16).color(TEXT_STRONG),
        overview_line("groups", state.storage.group_count()),
        overview_line("history", state.storage.command_history_count()),
        overview_line("known hosts", state.storage.known_host_count()),
        overview_line("bookmarks", state.storage.sftp_bookmark_count()),
        overview_line("workspace tabs", state.storage.workspace_tab_count()),
        overview_line("local shells", state.terminal.local_shell_count()),
    ]
    .spacing(6)
    .into()
}

fn overview_line(label: &'static str, value: usize) -> Element<'static, Message> {
    row![
        text(label).size(12).color(TEXT_MUTED).width(Length::Fill),
        text(value.to_string()).size(12).color(TEXT_SOFT),
    ]
    .spacing(8)
    .into()
}

const TEXT_STRONG: Color = Color::from_rgb8(232, 238, 247);
const TEXT_SOFT: Color = Color::from_rgb8(199, 210, 224);
const TEXT_MUTED: Color = Color::from_rgb8(137, 149, 166);
const ACCENT: Color = Color::from_rgb8(72, 191, 145);
const PANEL: Color = Color::from_rgb8(24, 30, 40);
const PANEL_ELEVATED: Color = Color::from_rgb8(31, 39, 51);
const PANEL_ALT: Color = Color::from_rgb8(18, 24, 34);
const BORDER: Color = Color::from_rgb8(47, 59, 76);

fn app_background_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(10, 14, 20))
        .color(TEXT_SOFT)
}

fn top_bar_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(15, 20, 29))
        .border(Border::default().width(1).color(BORDER))
}

fn sidebar_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(PANEL_ALT)
        .border(Border::default().width(1).color(BORDER))
}

fn selected_nav_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(25, 45, 43))
        .border(Border::default().rounded(6).width(1).color(ACCENT))
}

fn transparent_style(_: &Theme) -> container::Style {
    container::Style::default()
}

fn error_banner_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(86, 43, 32))
        .border(
            Border::default()
                .width(1)
                .color(Color::from_rgb8(154, 78, 54)),
        )
}

fn workbench_style(_: &Theme) -> container::Style {
    container::Style::default().background(Color::from_rgb8(12, 17, 25))
}

fn inspector_style(_: &Theme) -> container::Style {
    container::Style::default().background(PANEL_ALT)
}

fn hero_style(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_SOFT),
        background: Some(Background::Color(Color::from_rgb8(22, 35, 43))),
        border: Border::default().rounded(8).width(1).color(BORDER),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.25),
            offset: Vector::new(0.0, 6.0),
            blur_radius: 18.0,
        },
        snap: false,
    }
}

fn metric_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(PANEL)
        .border(Border::default().rounded(8).width(1).color(BORDER))
}

fn panel_style(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TEXT_SOFT),
        background: Some(Background::Color(PANEL_ELEVATED)),
        border: Border::default().rounded(8).width(1).color(BORDER),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.20),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 14.0,
        },
        snap: false,
    }
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

    #[test]
    fn view_accepts_state_with_last_error() {
        let mut state = AppState::default();
        state.ui.set_last_error("认证失败");

        let _element = view(&state);
    }
}
