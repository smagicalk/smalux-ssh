//! 命令面板搜索与渲染。

use iced::{
    Alignment, Element, Length,
    widget::{button, column, container, row, scrollable, text, text_input},
};

use crate::model::{AppState, Message, SnippetScope, WorkspacePage};

use super::{
    TEXT_MUTED, TEXT_SOFT, TEXT_STRONG, TEXT_SUBTLE, auth_user, empty_line, flat_button_style,
    ghost_button_style, input_style, list_item_style, t, tag_style,
};

pub(super) fn command_palette(state: &AppState) -> Element<'_, Message> {
    let query = state.ui.workspace.command_palette.query.clone();

    container(
        column![
            row![
                text(t(state, "palette.title"))
                    .size(15)
                    .color(TEXT_STRONG)
                    .width(Length::Fill),
                button(text(t(state, "common.close")).size(11))
                    .padding([6, 9])
                    .style(ghost_button_style)
                    .on_press(Message::CloseCommandPalette),
            ]
            .align_y(Alignment::Center),
            text_input(t(state, "palette.placeholder"), &query)
                .on_input(|query| Message::UpdateCommandPaletteQuery { query })
                .padding([9, 10])
                .style(input_style),
            command_palette_results(state),
        ]
        .spacing(10),
    )
    .padding(14)
    .width(Length::Fill)
    .style(super::command_palette_style)
    .into()
}

fn command_palette_results(state: &AppState) -> Element<'_, Message> {
    let query = state
        .ui
        .workspace
        .command_palette
        .query
        .trim()
        .to_ascii_lowercase();
    let has_query = !query.is_empty();
    let mut rows = column![].spacing(8);
    let mut count = 0usize;

    if matches_query(t(state, "palette.hosts"), &query, has_query) || has_query {
        rows = rows.push(palette_section_label(state, "palette.hosts"));
        for host in state
            .storage
            .hosts
            .iter()
            .filter(|host| {
                matches_query(&host.name, &query, has_query)
                    || matches_query(&host.address, &query, has_query)
                    || host
                        .tags
                        .iter()
                        .any(|tag| matches_query(tag, &query, has_query))
            })
            .take(4)
        {
            count += 1;
            rows = rows.push(palette_action_row(
                "Host",
                host.name.as_str(),
                format!("{}@{}:{}", auth_user(host), host.address, host.port),
                Some(Message::OpenShell { host_id: host.id }),
            ));
        }
    }

    rows = rows.push(palette_section_label(state, "palette.snippets"));
    for snippet in state
        .storage
        .snippets
        .iter()
        .filter(|snippet| {
            matches_query(&snippet.name, &query, has_query)
                || matches_query(&snippet.command_template, &query, has_query)
        })
        .take(4)
    {
        count += 1;
        rows = rows.push(palette_action_row(
            snippet_scope_label(&snippet.scope),
            snippet.name.as_str(),
            snippet.command_template.as_str(),
            None,
        ));
    }

    rows = rows.push(palette_section_label(state, "palette.history"));
    for item in state
        .storage
        .command_history
        .iter()
        .rev()
        .filter(|item| matches_query(&item.command, &query, has_query))
        .take(4)
    {
        count += 1;
        let hint = item
            .working_directory
            .as_deref()
            .unwrap_or("global command history");
        rows = rows.push(palette_action_row(
            "History",
            item.command.as_str(),
            hint,
            item.host_id.map(|_| Message::RunCommandHistory {
                history_id: item.id,
            }),
        ));
    }

    rows = rows.push(palette_section_label(state, "palette.settings"));
    for (key, hint) in [
        ("settings.general", "Language, startup, workspace"),
        ("settings.appearance", "Theme, palette, background carousel"),
        ("settings.ssh", "PTY, auth, agent, certificates"),
        ("settings.sftp", "Transfers, local and remote panes"),
        ("settings.terminal", "Font, shell, scrollback"),
        ("settings.security", "Known hosts, credentials, keychain"),
        ("settings.shortcuts", "Command palette and tabs"),
        ("settings.advanced", "Storage, logs, diagnostics"),
    ]
    .into_iter()
    .filter(|(key, hint)| {
        matches_query(t(state, key), &query, has_query) || matches_query(hint, &query, has_query)
    })
    .take(4)
    {
        count += 1;
        rows = rows.push(palette_action_row(
            "Settings",
            t(state, key),
            hint,
            Some(Message::SetWorkspacePage {
                page: WorkspacePage::Settings,
            }),
        ));
    }

    if count == 0 {
        rows = rows.push(empty_line(t(state, "palette.empty")));
    }

    scrollable(rows).height(160).into()
}

fn palette_section_label<'a>(state: &'a AppState, key: &'static str) -> Element<'a, Message> {
    text(t(state, key)).size(10).color(TEXT_SUBTLE).into()
}

fn palette_action_row(
    kind: &'static str,
    title: impl Into<String>,
    hint: impl Into<String>,
    message: Option<Message>,
) -> Element<'static, Message> {
    let title = title.into();
    let hint = hint.into();
    let content = row![
        container(text(kind).size(10).color(TEXT_STRONG))
            .padding([4, 7])
            .style(tag_style),
        column![
            text(title).size(12).color(TEXT_SOFT),
            text(hint).size(10).color(TEXT_MUTED),
        ]
        .spacing(2)
        .width(Length::Fill),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    match message {
        Some(message) => button(content)
            .padding(8)
            .width(Length::Fill)
            .style(flat_button_style)
            .on_press(message)
            .into(),
        None => container(content)
            .padding(8)
            .width(Length::Fill)
            .style(list_item_style)
            .into(),
    }
}

fn matches_query(value: &str, query: &str, has_query: bool) -> bool {
    !has_query || value.to_ascii_lowercase().contains(query)
}

fn snippet_scope_label(scope: &SnippetScope) -> &'static str {
    match scope {
        SnippetScope::Global => "Global",
        SnippetScope::Host(_) => "Host",
        SnippetScope::Group(_) => "Group",
    }
}
