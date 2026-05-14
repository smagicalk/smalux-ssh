//! 主界面视图组合。
//!
//! 这里只负责产品级首屏布局，业务操作仍委托到各功能视图模块。

use iced::{
    Alignment, Border, Color, Element, Length,
    widget::{Space, button, column, container, row, scrollable, text, text_input},
};

use crate::model::{
    AppState, Host, HostListMode, Message, QuickHostDraftField, SessionKind, SessionStatus,
    WorkspacePage,
};

mod command_palette;
#[allow(dead_code)]
mod host_actions;
mod i18n;
#[allow(dead_code)]
mod security;
#[allow(dead_code)]
mod session_summary;
#[allow(dead_code)]
mod sftp_workspace;
#[allow(dead_code)]
mod terminal_workspace;
mod theme;
#[allow(dead_code)]
mod visual_settings;
#[allow(dead_code)]
mod workspace;

use command_palette::command_palette;
use i18n::{background_label, host_list_mode_label, page_title, t, theme_name};
use theme::*;

/// 根据应用状态构建当前主界面。
pub fn view(state: &AppState) -> Element<'_, Message> {
    let mut root = column![title_bar(state)];
    if let Some(error) = state.ui.last_error.as_deref() {
        root = root.push(error_banner(error));
    }
    if state.ui.workspace.command_palette.open {
        root = root.push(command_palette(state));
    }

    root = root.push(
        row![
            nav_rail(state),
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
        .style(move |_| app_background_style_for(state))
        .into()
}

fn title_bar(state: &AppState) -> Element<'_, Message> {
    container(
        row![
            row![
                brand_mark(),
                column![
                    text(t(state, "app.name")).size(19).color(TEXT_STRONG),
                    text(t(state, "app.subtitle")).size(11).color(TEXT_MUTED),
                ]
                .spacing(1),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            page_tabs(state),
            Space::new().width(Length::Fill),
            command_search_button(state),
            status_pill(t(state, "top.hosts"), state.storage.host_count()),
            status_pill(t(state, "top.tabs"), state.sessions.tab_count()),
            toolbar_button(
                t(state, "top.new_connection"),
                Message::SetWorkspacePage {
                    page: WorkspacePage::Hosts,
                }
            ),
            toolbar_button(
                t(state, "nav.settings"),
                Message::SetWorkspacePage {
                    page: WorkspacePage::Settings,
                }
            ),
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

fn page_tabs(state: &AppState) -> Element<'_, Message> {
    row![
        page_tab(state, "nav.hosts", WorkspacePage::Hosts),
        page_tab(state, "nav.sftp", WorkspacePage::Sftp),
        page_tab(state, "nav.tunnels", WorkspacePage::Tunnels),
        page_tab(state, "nav.snippets", WorkspacePage::Snippets),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .into()
}

fn page_tab<'a>(
    state: &'a AppState,
    label_key: &'static str,
    page: WorkspacePage,
) -> Element<'a, Message> {
    let active = state.ui.workspace.active_page == page;
    button(
        container(text(t(state, label_key)).size(12).color(if active {
            TEXT_STRONG
        } else {
            TEXT_MUTED
        }))
        .padding([7, 12])
        .style(if active {
            active_tab_style
        } else {
            quiet_tab_style
        }),
    )
    .padding(0)
    .style(flat_button_style)
    .on_press(Message::SetWorkspacePage { page })
    .into()
}

fn command_search_button(state: &AppState) -> Element<'_, Message> {
    button(
        row![
            text("⌘K").size(11).color(TEXT_MUTED),
            text(t(state, "top.search")).size(12).color(TEXT_SOFT),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([7, 12])
    .style(ghost_button_style)
    .on_press(Message::OpenCommandPalette {
        query: String::new(),
    })
    .into()
}

fn status_pill(label: &str, value: usize) -> Element<'_, Message> {
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

fn toolbar_button(label: &str, message: Message) -> Element<'_, Message> {
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

fn nav_rail(state: &AppState) -> Element<'_, Message> {
    container(
        column![
            nav_icon(state, "H", "nav.hosts", WorkspacePage::Hosts),
            nav_icon(state, "T", "nav.terminal", WorkspacePage::Terminal),
            nav_icon(state, "F", "nav.sftp", WorkspacePage::Sftp),
            nav_icon(state, "P", "nav.tunnels", WorkspacePage::Tunnels),
            nav_icon(state, "S", "nav.snippets", WorkspacePage::Snippets),
            nav_icon(state, "R", "nav.history", WorkspacePage::History),
            Space::new().height(Length::Fill),
            nav_icon(state, "⚙", "nav.settings", WorkspacePage::Settings),
        ]
        .spacing(10)
        .align_x(Alignment::Center),
    )
    .width(64)
    .height(Length::Fill)
    .padding([14, 8])
    .style(nav_rail_style)
    .into()
}

fn nav_icon<'a>(
    state: &'a AppState,
    glyph: &'static str,
    label_key: &'static str,
    page: WorkspacePage,
) -> Element<'a, Message> {
    let active = state.ui.workspace.active_page == page;
    button(
        column![
            text(glyph)
                .size(14)
                .color(if active { Color::WHITE } else { TEXT_MUTED }),
            text(t(state, label_key))
                .size(9)
                .color(if active { TEXT_STRONG } else { TEXT_SUBTLE }),
        ]
        .spacing(2)
        .align_x(Alignment::Center),
    )
    .padding([7, 4])
    .width(48)
    .style(if active {
        primary_button_style
    } else {
        flat_button_style
    })
    .on_press(Message::SetWorkspacePage { page })
    .into()
}

fn connection_rail(state: &AppState) -> Element<'_, Message> {
    container(
        column![
            rail_header(state),
            quick_connect(state),
            section_label(t(state, "hosts.groups")),
            group_tree(state),
            hosts_filter_bar(state),
            section_label(t(state, "hosts.hosts")),
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

fn rail_header(state: &AppState) -> Element<'_, Message> {
    row![
        column![
            text(page_title(state)).size(16).color(TEXT_STRONG),
            text(t(state, "hosts.filter_hint"))
                .size(10)
                .color(TEXT_MUTED),
        ]
        .spacing(2)
        .width(Length::Fill),
        button(text(host_list_mode_label(state)).size(10))
            .padding([6, 9])
            .style(ghost_button_style)
            .on_press(Message::ToggleHostListMode),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn hosts_filter_bar(state: &AppState) -> Element<'_, Message> {
    row![
        filter_chip(t(state, "hosts.filter_group"), true),
        filter_chip(t(state, "hosts.filter_tags"), false),
        filter_chip(t(state, "hosts.filter_status"), false),
    ]
    .spacing(6)
    .into()
}

fn filter_chip(label: &str, active: bool) -> Element<'_, Message> {
    container(
        text(label)
            .size(10)
            .color(if active { TEXT_STRONG } else { TEXT_MUTED }),
    )
    .padding([5, 8])
    .style(if active { active_tab_style } else { pill_style })
    .into()
}

fn quick_connect(state: &AppState) -> Element<'_, Message> {
    let draft = &state.ui.quick_host;

    container(
        column![
            row![
                text(t(state, "hosts.quick_connect"))
                    .size(15)
                    .color(TEXT_STRONG),
                Space::new().width(Length::Fill),
                text("⌘K").size(11).color(TEXT_MUTED),
            ]
            .align_y(Alignment::Center),
            text_input(t(state, "hosts.host_or_alias"), &draft.address)
                .on_input(|value| Message::UpdateQuickHostDraft {
                    field: QuickHostDraftField::Address,
                    value,
                })
                .size(13)
                .padding([9, 10])
                .style(input_style),
            row![
                text_input(t(state, "hosts.user"), &draft.username)
                    .on_input(|value| Message::UpdateQuickHostDraft {
                        field: QuickHostDraftField::Username,
                        value,
                    })
                    .size(13)
                    .padding([9, 10])
                    .style(input_style),
                button(text(t(state, "common.save")).size(12))
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

fn section_label(label: &str) -> Element<'_, Message> {
    text(label).size(10).color(TEXT_SUBTLE).into()
}

fn group_tree(state: &AppState) -> Element<'_, Message> {
    let mut groups = column![group_row(
        t(state, "hosts.all_connections"),
        state.storage.host_count(),
        true
    )]
    .spacing(6);

    if state.storage.groups.is_empty() {
        groups = groups.push(group_row(
            t(state, "hosts.ungrouped"),
            state.storage.host_count(),
            false,
        ));
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
        list = list.push(empty_host_card(state));
    } else {
        for host in &state.storage.hosts {
            list = match state.ui.workspace.host_list_mode {
                HostListMode::List => list.push(host_row(state, host)),
                HostListMode::Card => list.push(host_card_large(state, host)),
            };
        }
    }

    scrollable(list).height(Length::Fill).into()
}

fn empty_host_card(state: &AppState) -> Element<'_, Message> {
    container(
        column![
            text(t(state, "hosts.empty_title"))
                .size(14)
                .color(TEXT_SOFT),
            text(t(state, "hosts.empty_body"))
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
        t(state, "hosts.no_tags").to_owned()
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
                    t(state, "tool.run_short"),
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

fn host_card_large<'a>(state: &'a AppState, host: &'a Host) -> Element<'a, Message> {
    let host_id = host.id;
    let command = state.ui.remote_command_for(host_id).to_owned();

    container(
        column![
            row![
                host_badge(host.name.chars().next().unwrap_or('H')),
                column![
                    text(&host.name).size(15).color(TEXT_STRONG),
                    text(format!(
                        "{}@{}:{}",
                        auth_user(host),
                        host.address,
                        host.port
                    ))
                    .size(11)
                    .color(TEXT_MUTED),
                ]
                .spacing(3)
                .width(Length::Fill),
                connection_status_dot(state, host_id),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            tag_row(state, host),
            row![
                small_action(t(state, "tool.connect"), Message::OpenShell { host_id }),
                small_action(
                    "SFTP",
                    Message::OpenSftp {
                        host_id,
                        initial_dir: state.ui.sftp_initial_dir_for(host_id).to_owned(),
                    },
                ),
                small_action(
                    t(state, "tool.run"),
                    Message::RunRemoteCommand {
                        host_id,
                        command,
                        request_pty: false,
                    },
                ),
            ]
            .spacing(6),
        ]
        .spacing(10),
    )
    .padding(13)
    .width(Length::Fill)
    .style(host_card_style)
    .into()
}

fn tag_row<'a>(state: &'a AppState, host: &'a Host) -> Element<'a, Message> {
    if host.tags.is_empty() {
        return text(t(state, "hosts.no_tags"))
            .size(11)
            .color(TEXT_SUBTLE)
            .into();
    }

    let mut row = row![].spacing(5);
    for tag in host.tags.iter().take(3) {
        row = row.push(
            container(text(tag).size(10).color(TEXT_STRONG))
                .padding([4, 7])
                .style(tag_style),
        );
    }

    row.into()
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

fn small_action(label: &str, message: Message) -> Element<'_, Message> {
    button(text(label).size(11))
        .padding([6, 9])
        .style(ghost_button_style)
        .on_press(message)
        .into()
}

fn command_workspace(state: &AppState) -> Element<'_, Message> {
    let content: Element<'_, Message> = match state.ui.workspace.active_page {
        WorkspacePage::Hosts | WorkspacePage::Terminal => host_terminal_workspace(state),
        WorkspacePage::Sftp => sftp_page_workspace(state),
        WorkspacePage::Tunnels => placeholder_workspace(state, "nav.tunnels", "settings.ssh"),
        WorkspacePage::Snippets => placeholder_workspace(state, "nav.snippets", "snippet.subtitle"),
        WorkspacePage::History => placeholder_workspace(state, "nav.history", "history.subtitle"),
        WorkspacePage::Security => {
            placeholder_workspace(state, "nav.security", "settings.security")
        }
        WorkspacePage::Settings => settings_workspace(state),
    };

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(workspace_style)
        .into()
}

fn host_terminal_workspace(state: &AppState) -> Element<'_, Message> {
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
    .padding(16)
    .into()
}

fn sftp_page_workspace(state: &AppState) -> Element<'_, Message> {
    column![
        workspace_page_header(state, "nav.sftp", "sftp.page_subtitle"),
        container(
            row![sftp_local_panel(state), sftp_remote_panel(state)]
                .spacing(14)
                .height(Length::Fill),
        )
        .height(Length::Fill),
    ]
    .spacing(12)
    .padding(16)
    .into()
}

fn placeholder_workspace<'a>(
    state: &'a AppState,
    title_key: &'static str,
    subtitle_key: &'static str,
) -> Element<'a, Message> {
    column![
        workspace_page_header(state, title_key, subtitle_key),
        container(
            column![
                text(t(state, "common.coming_soon"))
                    .size(22)
                    .color(TEXT_STRONG),
                text(t(state, "common.layout_reserved"))
                    .size(13)
                    .color(TEXT_MUTED),
            ]
            .spacing(8)
            .align_x(Alignment::Center),
        )
        .center(Length::Fill)
        .height(Length::Fill)
        .style(side_panel_style),
    ]
    .spacing(12)
    .padding(16)
    .into()
}

fn settings_workspace(state: &AppState) -> Element<'_, Message> {
    column![
        workspace_page_header(state, "nav.settings", "settings.subtitle"),
        row![
            settings_category(
                state,
                "settings.general",
                state.ui.workspace.language_label()
            ),
            settings_category(
                state,
                "settings.appearance",
                theme_name(state.ui.workspace.theme)
            ),
            settings_category(state, "settings.ssh", "russh"),
            settings_category(state, "settings.sftp", "dual layout"),
        ]
        .spacing(12),
        row![
            settings_category(state, "settings.terminal", "xterm-256color"),
            settings_category(state, "settings.security", "keyring"),
            settings_category(state, "settings.shortcuts", "Ctrl/Cmd+K"),
            settings_category(state, "settings.advanced", "redb"),
        ]
        .spacing(12),
        background_carousel_settings(state),
    ]
    .spacing(12)
    .padding(16)
    .into()
}

fn background_carousel_settings(state: &AppState) -> Element<'_, Message> {
    let background = state.config.background.normalized();
    let active_source = state
        .ui
        .workspace
        .active_background_index(background.sources.len())
        .and_then(|index| background.sources.get(index))
        .map(background_label)
        .unwrap_or("No background source")
        .to_owned();
    let summary = format!(
        "{} sources · opacity {:.0}% · blur {:.0}px · {}s",
        background.sources.len(),
        background.opacity * 100.0,
        background.blur,
        background.rotation_interval_secs,
    );

    container(
        row![
            column![
                text("Background Carousel").size(14).color(TEXT_STRONG),
                text(summary).size(11).color(TEXT_MUTED),
                text(active_source).size(11).color(TEXT_SOFT),
            ]
            .spacing(5)
            .width(Length::Fill),
            button(text("Next").size(11))
                .padding([7, 10])
                .style(ghost_button_style)
                .on_press(Message::NextBackground),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding(14)
    .width(Length::Fill)
    .style(side_panel_style)
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

fn workspace_page_header<'a>(
    state: &'a AppState,
    title_key: &'static str,
    subtitle_key: &'static str,
) -> Element<'a, Message> {
    row![
        column![
            text(t(state, title_key)).size(20).color(TEXT_STRONG),
            text(t(state, subtitle_key)).size(12).color(TEXT_MUTED),
        ]
        .spacing(3)
        .width(Length::Fill),
        toolbar_button(
            t(state, "top.search"),
            Message::OpenCommandPalette {
                query: String::new(),
            }
        ),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

fn sftp_local_panel(state: &AppState) -> Element<'_, Message> {
    let mut files = column![panel_title(t(state, "sftp.local"), 3)].spacing(8);
    for (name, kind) in [
        ("project/", "dir"),
        ("deploy.sh", "file"),
        ("artifact.tar", "file"),
    ] {
        files = files.push(file_row(name, kind));
    }

    container(files)
        .padding(14)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(side_panel_style)
        .into()
}

fn sftp_remote_panel(state: &AppState) -> Element<'_, Message> {
    let mut files = column![panel_title(
        t(state, "sftp.remote"),
        state.sessions.sftp_browser_count()
    )]
    .spacing(8);

    if let Some(browser) = state.sessions.sftp_browsers.first() {
        for entry in &browser.entries {
            files = files.push(file_row(
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
        files = files.push(empty_line(t(state, "sftp.empty")));
    }

    container(files)
        .padding(14)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(side_panel_style)
        .into()
}

fn settings_category<'a>(
    state: &'a AppState,
    key: &'static str,
    value: &'a str,
) -> Element<'a, Message> {
    container(
        column![
            text(t(state, key)).size(14).color(TEXT_STRONG),
            text(value).size(11).color(TEXT_MUTED),
        ]
        .spacing(6),
    )
    .padding(14)
    .width(Length::Fill)
    .style(side_panel_style)
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
            output = output.push(terminal_empty_state(state));
        } else {
            for line in lines.into_iter().rev() {
                output = output.push(text(line).size(12).color(TERMINAL_TEXT));
            }
        }
        output = output.push(shell_prompt(state, tab.session_id));
    } else {
        output = output
            .push(terminal_empty_state(state))
            .push(text("$ ssh deploy@production").size(12).color(TERMINAL_DIM))
            .push(
                text(t(state, "terminal.waiting"))
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

fn terminal_empty_state(state: &AppState) -> Element<'_, Message> {
    column![
        Space::new().height(Length::Fill),
        text(t(state, "terminal.empty_title"))
            .size(16)
            .color(TERMINAL_TEXT),
        text(t(state, "terminal.empty_body"))
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
        return text(t(state, "terminal.read_only"))
            .size(11)
            .color(TERMINAL_DIM)
            .into();
    }

    let draft = state.ui.terminal_input_for(session_id);
    row![
        text("❯").size(14).color(ACCENT),
        text_input(t(state, "terminal.send_input"), draft)
            .on_input(move |input| Message::UpdateTerminalInputDraft { session_id, input })
            .on_submit(Message::SendTerminalInput { session_id })
            .size(12)
            .padding([8, 10])
            .style(terminal_input_style)
            .width(Length::Fill),
        small_action(
            t(state, "terminal.send"),
            Message::SendTerminalInput { session_id }
        ),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn snippet_strip(state: &AppState) -> Element<'_, Message> {
    let mut snippets = column![
        panel_title(t(state, "nav.snippets"), state.storage.snippet_count()),
        text(t(state, "snippet.subtitle"))
            .size(11)
            .color(TEXT_MUTED),
    ]
    .spacing(8);

    if state.storage.snippets.is_empty() {
        snippets = snippets.push(empty_line(t(state, "snippet.empty")));
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
    let mut left = column![text(t(state, "sftp.local")).size(12).color(TEXT_MUTED)].spacing(6);
    let mut right = column![text(t(state, "sftp.remote")).size(12).color(TEXT_MUTED)].spacing(6);

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
            .push(file_row(t(state, "sftp.open_to_load"), "hint"));
    }

    container(
        column![
            panel_title(t(state, "nav.sftp"), state.sessions.sftp_browser_count()),
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
    if state.ui.workspace.right_sidebar_collapsed {
        return collapsed_activity_panel(state);
    }

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

fn collapsed_activity_panel(state: &AppState) -> Element<'_, Message> {
    container(
        column![
            button(text("›").size(18))
                .padding([8, 10])
                .style(primary_button_style)
                .on_press(Message::ToggleRightSidebar),
            text(t(state, "activity.collapsed"))
                .size(10)
                .color(TEXT_MUTED),
        ]
        .spacing(10)
        .align_x(Alignment::Center),
    )
    .width(42)
    .height(Length::Fill)
    .padding([14, 4])
    .style(activity_style)
    .into()
}

fn activity_header(state: &AppState) -> Element<'_, Message> {
    row![
        column![
            text(t(state, "activity.title")).size(16).color(TEXT_STRONG),
            text(format!(
                "{} {} · {} {} · {} {}",
                state.sessions.transfer_count(),
                t(state, "activity.transfers"),
                state.storage.tunnel_rule_count(),
                t(state, "activity.tunnel_rules"),
                state.storage.sftp_bookmark_count(),
                t(state, "activity.bookmarks"),
            ))
            .size(11)
            .color(TEXT_MUTED),
        ]
        .spacing(4)
        .width(Length::Fill),
        button(text("‹").size(14))
            .padding([6, 9])
            .style(ghost_button_style)
            .on_press(Message::ToggleRightSidebar),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn runtime_section(state: &AppState) -> Element<'_, Message> {
    let mut rows = column![section_label(t(state, "activity.runtime"))].spacing(7);

    if state.sessions.tabs.is_empty() {
        rows = rows.push(empty_line(t(state, "activity.no_sessions")));
    } else {
        for tab in state.sessions.tabs.iter().take(4) {
            let mut line = row![
                column![
                    text(&tab.title).size(12).color(TEXT_STRONG),
                    text(format!("{:?}", tab.status)).size(10).color(TEXT_MUTED),
                ]
                .spacing(2)
                .width(Length::Fill),
                button(text(t(state, "common.close")).size(10))
                    .padding([5, 8])
                    .style(ghost_button_style)
                    .on_press(Message::CloseSessionTab { session_id: tab.id }),
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            if let SessionKind::Tunnel { rule_name } = &tab.kind {
                line = line.push(
                    button(text(t(state, "common.stop")).size(10))
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
    let mut rows = column![section_label(t(state, "activity.recent"))].spacing(7);

    if state.storage.recent_connections.is_empty() {
        rows = rows.push(empty_line(t(state, "activity.no_recent")));
    } else {
        for item in state.storage.recent_connections.iter().take(4) {
            rows = rows.push(
                row![
                    text(&item.label)
                        .size(12)
                        .color(TEXT_SOFT)
                        .width(Length::Fill),
                    button(text(t(state, "common.open")).size(10))
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
    let mut rows = column![section_label(t(state, "activity.history"))].spacing(7);

    if state.storage.command_history.is_empty() {
        rows = rows.push(empty_line(t(state, "activity.no_history")));
    } else {
        for item in state.storage.command_history.iter().rev().take(4) {
            let run_button: Element<'_, Message> = if item.host_id.is_some() {
                button(text(t(state, "tool.run_short")).size(10))
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
            text(t(state, "activity.advanced"))
                .size(12)
                .color(TEXT_MUTED),
            compact_metric(
                t(state, "activity.credentials"),
                state.storage.credential_count()
            ),
            compact_metric(
                t(state, "activity.known_hosts"),
                state.storage.known_host_count()
            ),
            compact_metric(
                t(state, "activity.visual_profiles"),
                usize::from(state.config.background.enabled)
            ),
            compact_metric(
                t(state, "activity.workspace_tabs"),
                state.storage.workspace_tab_count()
            ),
        ]
        .spacing(6),
    )
    .padding(10)
    .style(side_panel_style)
    .into()
}

fn compact_metric(label: &str, value: usize) -> Element<'_, Message> {
    row![
        text(label).size(11).color(TEXT_MUTED).width(Length::Fill),
        text(value.to_string()).size(11).color(TEXT_SOFT),
    ]
    .spacing(8)
    .into()
}

fn panel_title(label: &str, count: usize) -> Element<'_, Message> {
    row![
        text(label).size(14).color(TEXT_STRONG).width(Length::Fill),
        text(count.to_string()).size(11).color(TEXT_MUTED),
    ]
    .align_y(Alignment::Center)
    .into()
}

fn empty_line(label: &str) -> Element<'_, Message> {
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
