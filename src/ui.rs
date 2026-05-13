//! 主界面视图组合。
//!
//! 这里只负责产品级首屏布局，业务操作仍委托到各功能视图模块。

use iced::{
    Alignment, Background, Border, Color, Element, Length, Shadow, Theme, Vector,
    widget::{Space, button, column, container, row, scrollable, text, text_input},
};

use crate::model::{AppState, Host, Message, QuickHostDraftField, SessionKind, SessionStatus};

#[allow(dead_code)]
mod host_actions;
#[allow(dead_code)]
mod security;
#[allow(dead_code)]
mod session_summary;
#[allow(dead_code)]
mod sftp_workspace;
#[allow(dead_code)]
mod terminal_workspace;
#[allow(dead_code)]
mod visual_settings;
#[allow(dead_code)]
mod workspace;

/// 根据应用状态构建当前主界面。
pub fn view(state: &AppState) -> Element<'_, Message> {
    let mut root = column![title_bar(state)];
    if let Some(error) = state.ui.last_error.as_deref() {
        root = root.push(error_banner(error));
    }

    root = root.push(
        row![
            connection_rail(state),
            vertical_rule(),
            command_workspace(state),
            activity_panel(state),
        ]
        .height(Length::Fill),
    );

    container(root)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(app_background_style)
        .into()
}

fn title_bar(state: &AppState) -> Element<'_, Message> {
    container(
        row![
            row![
                brand_mark(),
                column![
                    text("smagicalssh").size(19).color(TEXT_STRONG),
                    text("SSH · SFTP · Tunnels · Snippets")
                        .size(11)
                        .color(TEXT_MUTED),
                ]
                .spacing(1),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            protocol_tabs(),
            Space::new().width(Length::Fill),
            status_pill("Hosts", state.storage.host_count()),
            status_pill("Tabs", state.sessions.tab_count()),
            status_pill("Queue", state.backend_commands.pending_count()),
            toolbar_button("Theme", Message::ToggleTheme),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding([12, 18])
    .width(Length::Fill)
    .style(title_bar_style)
    .into()
}

fn brand_mark() -> Element<'static, Message> {
    container(text("S").size(18).color(Color::WHITE))
        .width(34)
        .height(34)
        .center(Length::Fill)
        .style(accent_block_style)
        .into()
}

fn protocol_tabs() -> Element<'static, Message> {
    row![
        protocol_tab("SSH", true),
        protocol_tab("SFTP", false),
        protocol_tab("Tunnel", false),
        protocol_tab("Snippet", false),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .into()
}

fn protocol_tab(label: &'static str, active: bool) -> Element<'static, Message> {
    container(
        text(label)
            .size(12)
            .color(if active { TEXT_STRONG } else { TEXT_MUTED }),
    )
    .padding([7, 12])
    .style(if active {
        active_tab_style
    } else {
        quiet_tab_style
    })
    .into()
}

fn status_pill(label: &'static str, value: usize) -> Element<'static, Message> {
    container(
        row![
            text(label).size(11).color(TEXT_MUTED),
            text(value.to_string()).size(12).color(TEXT_STRONG),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([7, 10])
    .style(pill_style)
    .into()
}

fn toolbar_button(label: &'static str, message: Message) -> Element<'static, Message> {
    button(text(label).size(12))
        .padding([7, 12])
        .style(primary_button_style)
        .on_press(message)
        .into()
}

fn error_banner(error: &str) -> Element<'_, Message> {
    container(
        row![
            text(format!("Error: {error}"))
                .size(12)
                .color(Color::from_rgb8(255, 230, 204))
                .width(Length::Fill),
            button("Dismiss")
                .padding([6, 10])
                .style(ghost_button_style)
                .on_press(Message::DismissUiError),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .padding([9, 18])
    .width(Length::Fill)
    .style(error_banner_style)
    .into()
}

fn connection_rail(state: &AppState) -> Element<'_, Message> {
    container(
        column![
            quick_connect(state),
            section_label("GROUPS"),
            group_tree(state),
            section_label("HOSTS"),
            host_list(state),
        ]
        .spacing(12),
    )
    .width(310)
    .height(Length::Fill)
    .padding(16)
    .style(rail_style)
    .into()
}

fn quick_connect(state: &AppState) -> Element<'_, Message> {
    let draft = &state.ui.quick_host;

    container(
        column![
            row![
                text("Quick Connect").size(15).color(TEXT_STRONG),
                Space::new().width(Length::Fill),
                text("⌘K").size(11).color(TEXT_MUTED),
            ]
            .align_y(Alignment::Center),
            text_input("host or alias", &draft.address)
                .on_input(|value| Message::UpdateQuickHostDraft {
                    field: QuickHostDraftField::Address,
                    value,
                })
                .size(13)
                .padding([9, 10])
                .style(input_style),
            row![
                text_input("user", &draft.username)
                    .on_input(|value| Message::UpdateQuickHostDraft {
                        field: QuickHostDraftField::Username,
                        value,
                    })
                    .size(13)
                    .padding([9, 10])
                    .style(input_style),
                button(text("Save").size(12))
                    .padding([9, 12])
                    .style(primary_button_style)
                    .on_press(Message::SaveQuickHost),
            ]
            .spacing(8),
        ]
        .spacing(10),
    )
    .padding(12)
    .style(quick_connect_style)
    .into()
}

fn section_label(label: &'static str) -> Element<'static, Message> {
    text(label).size(10).color(TEXT_SUBTLE).into()
}

fn group_tree(state: &AppState) -> Element<'_, Message> {
    let mut groups = column![group_row(
        "All connections",
        state.storage.host_count(),
        true
    )]
    .spacing(6);

    if state.storage.groups.is_empty() {
        groups = groups.push(group_row("Ungrouped", state.storage.host_count(), false));
    } else {
        for group in &state.storage.groups {
            let count = state
                .storage
                .hosts
                .iter()
                .filter(|host| host.group_id == Some(group.id))
                .count();
            groups = groups.push(group_row(&group.name, count, false));
        }
    }

    groups.into()
}

fn group_row<'a>(label: &'a str, count: usize, active: bool) -> Element<'a, Message> {
    container(
        row![
            text(if active { "▾" } else { "▸" })
                .size(11)
                .color(TEXT_MUTED),
            text(label)
                .size(13)
                .color(if active { TEXT_STRONG } else { TEXT_SOFT })
                .width(Length::Fill),
            text(count.to_string()).size(11).color(TEXT_MUTED),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([7, 8])
    .style(if active {
        selected_row_style
    } else {
        transparent_style
    })
    .into()
}

fn host_list(state: &AppState) -> Element<'_, Message> {
    let mut list = column![].spacing(8);

    if state.storage.hosts.is_empty() {
        list = list.push(empty_host_card());
    } else {
        for host in &state.storage.hosts {
            list = list.push(host_row(state, host));
        }
    }

    scrollable(list).height(Length::Fill).into()
}

fn empty_host_card() -> Element<'static, Message> {
    container(
        column![
            text("No saved hosts").size(14).color(TEXT_SOFT),
            text("Create one from Quick Connect.")
                .size(12)
                .color(TEXT_MUTED),
        ]
        .spacing(4),
    )
    .padding(12)
    .width(Length::Fill)
    .style(host_card_style)
    .into()
}

fn host_row<'a>(state: &'a AppState, host: &'a Host) -> Element<'a, Message> {
    let host_id = host.id;
    let subtitle = format!("{}@{}:{}", auth_user(host), host.address, host.port);
    let tag_line = if host.tags.is_empty() {
        "no tags".to_owned()
    } else {
        host.tags.join(" · ")
    };
    let command = state.ui.remote_command_for(host_id).to_owned();

    container(
        column![
            row![
                host_badge(host.name.chars().next().unwrap_or('H')),
                column![
                    text(&host.name).size(14).color(TEXT_STRONG),
                    text(subtitle).size(11).color(TEXT_MUTED),
                ]
                .spacing(2)
                .width(Length::Fill),
                connection_status_dot(state, host_id),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            text(tag_line).size(11).color(TEXT_SUBTLE),
            row![
                small_action("Shell", Message::OpenShell { host_id }),
                small_action(
                    "SFTP",
                    Message::OpenSftp {
                        host_id,
                        initial_dir: state.ui.sftp_initial_dir_for(host_id).to_owned(),
                    }
                ),
                small_action(
                    "Run",
                    Message::RunRemoteCommand {
                        host_id,
                        command,
                        request_pty: false,
                    }
                ),
            ]
            .spacing(6),
        ]
        .spacing(9),
    )
    .padding(11)
    .width(Length::Fill)
    .style(host_card_style)
    .into()
}

fn host_badge(initial: char) -> Element<'static, Message> {
    container(text(initial.to_string()).size(13).color(TEXT_STRONG))
        .width(30)
        .height(30)
        .center(Length::Fill)
        .style(badge_style)
        .into()
}

fn connection_status_dot(state: &AppState, host_id: crate::model::HostId) -> Element<'_, Message> {
    let connected =
        state.sessions.tabs.iter().any(|tab| {
            tab.host_id == Some(host_id) && matches!(tab.status, SessionStatus::Connected)
        });
    let label = if connected { "online" } else { "ready" };

    container(
        text(label)
            .size(10)
            .color(if connected { ACCENT } else { TEXT_MUTED }),
    )
    .padding([5, 8])
    .style(pill_style)
    .into()
}

fn small_action(label: &'static str, message: Message) -> Element<'static, Message> {
    button(text(label).size(11))
        .padding([6, 9])
        .style(ghost_button_style)
        .on_press(message)
        .into()
}

fn command_workspace(state: &AppState) -> Element<'_, Message> {
    container(
        column![
            workspace_tabs(state),
            row![
                terminal_stage(state),
                column![snippet_strip(state), sftp_preview(state),]
                    .spacing(12)
                    .width(360),
            ]
            .spacing(14)
            .height(Length::Fill),
        ]
        .spacing(12)
        .padding(16),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .style(workspace_style)
    .into()
}

fn workspace_tabs(state: &AppState) -> Element<'_, Message> {
    let mut tabs = row![].spacing(7).align_y(Alignment::Center);

    if state.sessions.tabs.is_empty() {
        tabs = tabs.push(tab_chip("New Session", "ready", true, None));
    } else {
        for tab in &state.sessions.tabs {
            tabs = tabs.push(tab_chip(
                &tab.title,
                session_kind_label(&tab.kind),
                state.sessions.active_tab == Some(tab.id),
                Some(tab.id),
            ));
        }
    }

    row![
        tabs.width(Length::Fill),
        toolbar_button("Save workspace", Message::SaveWorkspaceSnapshot),
        toolbar_button("Restore", Message::RestoreWorkspaceSnapshot),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

fn tab_chip<'a>(
    title: &'a str,
    hint: &'a str,
    active: bool,
    session_id: Option<crate::model::SessionId>,
) -> Element<'a, Message> {
    let chip = row![
        text(title)
            .size(12)
            .color(if active { TEXT_STRONG } else { TEXT_SOFT }),
        text(hint).size(10).color(TEXT_MUTED),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let chip = container(chip).padding([8, 12]).style(if active {
        active_tab_style
    } else {
        quiet_tab_style
    });

    match session_id {
        Some(session_id) => button(chip)
            .padding(0)
            .style(flat_button_style)
            .on_press(Message::ActivateTerminalTab { session_id })
            .into(),
        None => chip.into(),
    }
}

fn terminal_stage(state: &AppState) -> Element<'_, Message> {
    let active_tab = state.terminal.active_tab.and_then(|session_id| {
        state
            .terminal
            .tabs
            .iter()
            .find(|tab| tab.session_id == session_id)
    });

    let mut output = column![terminal_header(active_tab.map(|tab| tab.title.as_str()))]
        .spacing(8)
        .height(Length::Fill);

    if let Some(tab) = active_tab {
        let lines: Vec<&String> = tab.buffer.iter().rev().take(18).collect();
        if lines.is_empty() {
            output = output.push(terminal_empty_state());
        } else {
            for line in lines.into_iter().rev() {
                output = output.push(text(line).size(12).color(TERMINAL_TEXT));
            }
        }
        output = output.push(shell_prompt(state, tab.session_id));
    } else {
        output = output
            .push(terminal_empty_state())
            .push(text("$ ssh deploy@production").size(12).color(TERMINAL_DIM))
            .push(
                text("Waiting for a shell session...")
                    .size(12)
                    .color(TERMINAL_TEXT),
            );
    }

    container(output)
        .padding(16)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(terminal_style)
        .into()
}

fn terminal_header(active_title: Option<&str>) -> Element<'_, Message> {
    row![
        terminal_dot(Color::from_rgb8(255, 95, 86)),
        terminal_dot(Color::from_rgb8(255, 189, 46)),
        terminal_dot(Color::from_rgb8(39, 201, 63)),
        text(active_title.unwrap_or("Terminal"))
            .size(12)
            .color(TERMINAL_DIM)
            .width(Length::Fill),
        text("PTY xterm-256color").size(11).color(TERMINAL_DIM),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn terminal_dot(color: Color) -> Element<'static, Message> {
    container(Space::new())
        .width(10)
        .height(10)
        .style(move |_| {
            container::Style::default()
                .background(color)
                .border(Border::default().rounded(5))
        })
        .into()
}

fn terminal_empty_state() -> Element<'static, Message> {
    column![
        Space::new().height(Length::Fill),
        text("No active terminal output")
            .size(16)
            .color(TERMINAL_TEXT),
        text("Open a host shell or run a command from the connection list.")
            .size(12)
            .color(TERMINAL_DIM),
        Space::new().height(Length::Fill),
    ]
    .spacing(8)
    .align_x(Alignment::Center)
    .into()
}

fn shell_prompt(state: &AppState, session_id: crate::model::SessionId) -> Element<'_, Message> {
    let is_shell = state
        .sessions
        .tabs
        .iter()
        .find(|session| session.id == session_id)
        .map(|session| matches!(session.kind, SessionKind::Shell))
        .unwrap_or(false);

    if !is_shell {
        return text("Remote command output is read-only.")
            .size(11)
            .color(TERMINAL_DIM)
            .into();
    }

    let draft = state.ui.terminal_input_for(session_id);
    row![
        text("❯").size(14).color(ACCENT),
        text_input("send input", draft)
            .on_input(move |input| Message::UpdateTerminalInputDraft { session_id, input })
            .on_submit(Message::SendTerminalInput { session_id })
            .size(12)
            .padding([8, 10])
            .style(terminal_input_style)
            .width(Length::Fill),
        small_action("Send", Message::SendTerminalInput { session_id }),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn snippet_strip(state: &AppState) -> Element<'_, Message> {
    let mut snippets = column![
        panel_title("Snippets", state.storage.snippet_count()),
        text("Reusable commands with variables")
            .size(11)
            .color(TEXT_MUTED),
    ]
    .spacing(8);

    if state.storage.snippets.is_empty() {
        snippets = snippets.push(empty_line("Save a command as a snippet from a host."));
    } else {
        for snippet in state.storage.snippets.iter().take(4) {
            snippets = snippets.push(
                container(
                    column![
                        text(&snippet.name).size(13).color(TEXT_STRONG),
                        text(&snippet.command_template).size(11).color(TEXT_MUTED),
                    ]
                    .spacing(3),
                )
                .padding([8, 10])
                .width(Length::Fill)
                .style(list_item_style),
            );
        }
    }

    container(snippets)
        .padding(12)
        .width(Length::Fill)
        .style(side_panel_style)
        .into()
}

fn sftp_preview(state: &AppState) -> Element<'_, Message> {
    let mut left = column![text("Local").size(12).color(TEXT_MUTED)].spacing(6);
    let mut right = column![text("Remote").size(12).color(TEXT_MUTED)].spacing(6);

    left = left
        .push(file_row("project/", "dir"))
        .push(file_row("deploy.sh", "file"))
        .push(file_row("artifact.tar", "file"));

    if let Some(browser) = state.sessions.sftp_browsers.first() {
        for entry in browser.entries.iter().take(4) {
            right = right.push(file_row(
                &entry.name,
                match entry.kind {
                    crate::model::SftpEntryKind::Directory => "dir",
                    crate::model::SftpEntryKind::File => "file",
                    crate::model::SftpEntryKind::Symlink => "link",
                    crate::model::SftpEntryKind::Other => "other",
                },
            ));
        }
    } else {
        right = right
            .push(file_row("/home", "dir"))
            .push(file_row("/var/log", "dir"))
            .push(file_row("open SFTP to load", "hint"));
    }

    container(
        column![
            panel_title("SFTP", state.sessions.sftp_browser_count()),
            row![left.width(Length::Fill), right.width(Length::Fill)].spacing(10),
        ]
        .spacing(10),
    )
    .padding(12)
    .width(Length::Fill)
    .style(side_panel_style)
    .into()
}

fn file_row<'a>(name: &'a str, kind: &'static str) -> Element<'a, Message> {
    row![
        text(match kind {
            "dir" => "▣",
            "file" => "·",
            "link" => "↗",
            _ => "·",
        })
        .size(11)
        .color(ACCENT),
        text(name).size(11).color(TEXT_SOFT).width(Length::Fill),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .into()
}

fn activity_panel(state: &AppState) -> Element<'_, Message> {
    container(
        column![
            activity_header(state),
            runtime_section(state),
            recent_section(state),
            history_section(state),
            Space::new().height(Length::Fill),
            compact_links(state),
        ]
        .spacing(14)
        .padding(14),
    )
    .width(330)
    .height(Length::Fill)
    .style(activity_style)
    .into()
}

fn activity_header(state: &AppState) -> Element<'_, Message> {
    column![
        text("Activity").size(16).color(TEXT_STRONG),
        text(format!(
            "{} transfers · {} tunnel rules · {} bookmarks",
            state.sessions.transfer_count(),
            state.storage.tunnel_rule_count(),
            state.storage.sftp_bookmark_count()
        ))
        .size(11)
        .color(TEXT_MUTED),
    ]
    .spacing(4)
    .into()
}

fn runtime_section(state: &AppState) -> Element<'_, Message> {
    let mut rows = column![section_label("RUNTIME")].spacing(7);

    if state.sessions.tabs.is_empty() {
        rows = rows.push(empty_line("No running sessions."));
    } else {
        for tab in state.sessions.tabs.iter().take(4) {
            let mut line = row![
                column![
                    text(&tab.title).size(12).color(TEXT_STRONG),
                    text(format!("{:?}", tab.status)).size(10).color(TEXT_MUTED),
                ]
                .spacing(2)
                .width(Length::Fill),
                button(text("Close").size(10))
                    .padding([5, 8])
                    .style(ghost_button_style)
                    .on_press(Message::CloseSessionTab { session_id: tab.id }),
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            if let SessionKind::Tunnel { rule_name } = &tab.kind {
                line = line.push(
                    button(text("Stop").size(10))
                        .padding([5, 8])
                        .style(danger_button_style)
                        .on_press(Message::StopTunnel {
                            session_id: tab.id,
                            rule_name: rule_name.clone(),
                        }),
                );
            }

            rows = rows.push(container(line).padding(9).style(list_item_style));
        }
    }

    rows.into()
}

fn recent_section(state: &AppState) -> Element<'_, Message> {
    let mut rows = column![section_label("RECENT")].spacing(7);

    if state.storage.recent_connections.is_empty() {
        rows = rows.push(empty_line("Recent connections appear here."));
    } else {
        for item in state.storage.recent_connections.iter().take(4) {
            rows = rows.push(
                row![
                    text(&item.label)
                        .size(12)
                        .color(TEXT_SOFT)
                        .width(Length::Fill),
                    button(text("Open").size(10))
                        .padding([5, 8])
                        .style(ghost_button_style)
                        .on_press(Message::OpenRecentConnection {
                            host_id: item.host_id,
                        }),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
    }

    rows.into()
}

fn history_section(state: &AppState) -> Element<'_, Message> {
    let mut rows = column![section_label("COMMAND HISTORY")].spacing(7);

    if state.storage.command_history.is_empty() {
        rows = rows.push(empty_line("Run commands to build history."));
    } else {
        for item in state.storage.command_history.iter().rev().take(4) {
            let run_button: Element<'_, Message> = if item.host_id.is_some() {
                button(text("Run").size(10))
                    .padding([5, 8])
                    .style(ghost_button_style)
                    .on_press(Message::RunCommandHistory {
                        history_id: item.id,
                    })
                    .into()
            } else {
                Space::new().width(Length::Shrink).into()
            };
            rows = rows.push(
                row![
                    text(&item.command)
                        .size(11)
                        .color(TEXT_SOFT)
                        .width(Length::Fill),
                    run_button,
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
    }

    rows.into()
}

fn compact_links(state: &AppState) -> Element<'_, Message> {
    container(
        column![
            text("Advanced panels").size(12).color(TEXT_MUTED),
            compact_metric("credentials", state.storage.credential_count()),
            compact_metric("known hosts", state.storage.known_host_count()),
            compact_metric(
                "visual profiles",
                usize::from(state.config.background.enabled)
            ),
            compact_metric("workspace tabs", state.storage.workspace_tab_count()),
        ]
        .spacing(6),
    )
    .padding(10)
    .style(side_panel_style)
    .into()
}

fn compact_metric(label: &'static str, value: usize) -> Element<'static, Message> {
    row![
        text(label).size(11).color(TEXT_MUTED).width(Length::Fill),
        text(value.to_string()).size(11).color(TEXT_SOFT),
    ]
    .spacing(8)
    .into()
}

fn panel_title(label: &'static str, count: usize) -> Element<'static, Message> {
    row![
        text(label).size(14).color(TEXT_STRONG).width(Length::Fill),
        text(count.to_string()).size(11).color(TEXT_MUTED),
    ]
    .align_y(Alignment::Center)
    .into()
}

fn empty_line(label: &'static str) -> Element<'static, Message> {
    text(label).size(11).color(TEXT_MUTED).into()
}

fn session_kind_label(kind: &SessionKind) -> &'static str {
    match kind {
        SessionKind::Shell => "shell",
        SessionKind::RemoteCommand { .. } => "command",
        SessionKind::Sftp => "sftp",
        SessionKind::Tunnel { .. } => "tunnel",
    }
}

fn auth_user(host: &Host) -> &str {
    match &host.auth {
        crate::model::AuthProfile::Password { username, .. }
        | crate::model::AuthProfile::Key { username, .. }
        | crate::model::AuthProfile::Agent { username, .. }
        | crate::model::AuthProfile::Certificate { username, .. } => username,
    }
}

fn vertical_rule() -> Element<'static, Message> {
    container(Space::new())
        .width(1)
        .height(Length::Fill)
        .style(rule_style)
        .into()
}

const TEXT_STRONG: Color = Color::from_rgb8(236, 240, 246);
const TEXT_SOFT: Color = Color::from_rgb8(199, 208, 219);
const TEXT_MUTED: Color = Color::from_rgb8(127, 139, 153);
const TEXT_SUBTLE: Color = Color::from_rgb8(91, 103, 118);
const ACCENT: Color = Color::from_rgb8(47, 201, 146);
const BLUE: Color = Color::from_rgb8(94, 151, 246);
const SURFACE: Color = Color::from_rgb8(18, 23, 31);
const SURFACE_2: Color = Color::from_rgb8(24, 31, 41);
const BORDER: Color = Color::from_rgb8(47, 58, 74);
const TERMINAL_BG: Color = Color::from_rgb8(6, 10, 14);
const TERMINAL_TEXT: Color = Color::from_rgb8(198, 238, 214);
const TERMINAL_DIM: Color = Color::from_rgb8(95, 120, 112);

fn app_background_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(11, 15, 21))
        .color(TEXT_SOFT)
}

fn title_bar_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(14, 19, 26))
        .border(Border::default().width(1).color(BORDER))
}

fn rail_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(13, 18, 25))
        .border(Border::default().width(1).color(BORDER))
}

fn workspace_style(_: &Theme) -> container::Style {
    container::Style::default().background(Color::from_rgb8(11, 15, 21))
}

fn activity_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(14, 19, 26))
        .border(Border::default().width(1).color(BORDER))
}

fn quick_connect_style(_: &Theme) -> container::Style {
    elevated_style(SURFACE_2, 8)
}

fn host_card_style(_: &Theme) -> container::Style {
    elevated_style(SURFACE, 8).border(Border::default().rounded(8).width(1).color(BORDER))
}

fn side_panel_style(_: &Theme) -> container::Style {
    elevated_style(SURFACE_2, 8)
}

fn terminal_style(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TERMINAL_TEXT),
        background: Some(Background::Color(TERMINAL_BG)),
        border: Border::default()
            .rounded(8)
            .width(1)
            .color(Color::from_rgb8(32, 50, 48)),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.35),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 24.0,
        },
        snap: false,
    }
}

fn list_item_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(21, 27, 36))
        .border(Border::default().rounded(6).width(1).color(BORDER))
}

fn selected_row_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(23, 44, 42))
        .border(Border::default().rounded(6).width(1).color(ACCENT))
}

fn transparent_style(_: &Theme) -> container::Style {
    container::Style::default()
}

fn accent_block_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(ACCENT)
        .border(Border::default().rounded(8))
}

fn badge_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(34, 47, 59))
        .border(Border::default().rounded(8).width(1).color(BORDER))
}

fn active_tab_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(28, 43, 49))
        .border(Border::default().rounded(8).width(1).color(ACCENT))
}

fn quiet_tab_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(SURFACE)
        .border(Border::default().rounded(8).width(1).color(BORDER))
}

fn pill_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(20, 26, 35))
        .border(Border::default().rounded(999).width(1).color(BORDER))
}

fn rule_style(_: &Theme) -> container::Style {
    container::Style::default().background(BORDER)
}

fn error_banner_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(78, 40, 32))
        .border(
            Border::default()
                .width(1)
                .color(Color::from_rgb8(144, 68, 48)),
        )
}

fn elevated_style(background: Color, radius: u8) -> container::Style {
    container::Style {
        text_color: Some(TEXT_SOFT),
        background: Some(Background::Color(background)),
        border: Border::default().rounded(radius).width(1).color(BORDER),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.20),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 14.0,
        },
        snap: false,
    }
}

fn primary_button_style(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb8(61, 215, 160),
        _ => ACCENT,
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::from_rgb8(3, 20, 16),
        border: Border::default().rounded(6),
        shadow: Shadow::default(),
        snap: false,
    }
}

fn ghost_button_style(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb8(34, 45, 58),
        _ => Color::from_rgb8(23, 30, 40),
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: TEXT_SOFT,
        border: Border::default().rounded(6).width(1).color(BORDER),
        shadow: Shadow::default(),
        snap: false,
    }
}

fn danger_button_style(_: &Theme, _: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(Color::from_rgb8(74, 35, 35))),
        text_color: Color::from_rgb8(255, 196, 196),
        border: Border::default()
            .rounded(6)
            .width(1)
            .color(Color::from_rgb8(129, 56, 56)),
        shadow: Shadow::default(),
        snap: false,
    }
}

fn flat_button_style(_: &Theme, _: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: TEXT_SOFT,
        border: Border::default(),
        shadow: Shadow::default(),
        snap: false,
    }
}

fn input_style(_: &Theme, status: text_input::Status) -> text_input::Style {
    let border_color = match status {
        text_input::Status::Focused { .. } => ACCENT,
        text_input::Status::Hovered => BLUE,
        _ => BORDER,
    };

    text_input::Style {
        background: Background::Color(Color::from_rgb8(12, 17, 24)),
        border: Border::default().rounded(6).width(1).color(border_color),
        icon: TEXT_MUTED,
        placeholder: TEXT_SUBTLE,
        value: TEXT_STRONG,
        selection: Color::from_rgb8(42, 94, 78),
    }
}

fn terminal_input_style(_: &Theme, status: text_input::Status) -> text_input::Style {
    let border_color = match status {
        text_input::Status::Focused { .. } => ACCENT,
        _ => Color::from_rgb8(26, 47, 43),
    };

    text_input::Style {
        background: Background::Color(Color::from_rgb8(5, 12, 13)),
        border: Border::default().rounded(6).width(1).color(border_color),
        icon: TERMINAL_DIM,
        placeholder: TERMINAL_DIM,
        value: TERMINAL_TEXT,
        selection: Color::from_rgb8(25, 82, 62),
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
