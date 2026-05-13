//! 终端工作区视图。
//!
//! 这里直接展示当前终端标签页和缓冲输出，作为交互式 shell 的最小可见入口。

use iced::{
    Element, Length,
    widget::{button, column, row, text, text_input},
};

use crate::model::{AppState, Message, SessionKind};

const OUTPUT_PREVIEW_LINES: usize = 12;

/// 渲染终端工作区。
pub fn view(state: &AppState) -> Element<'_, Message> {
    column![
        text("Terminal").size(22),
        terminal_tabs(state),
        active_terminal_output(state),
    ]
    .spacing(12)
    .into()
}

fn terminal_tabs(state: &AppState) -> Element<'_, Message> {
    let tabs = column![text("Open tabs").size(18)].spacing(8);

    if state.terminal.tabs.is_empty() {
        return tabs.push(text("No terminal tabs open.")).into();
    }

    let mut tab_row = row![].spacing(8);
    for tab in &state.terminal.tabs {
        let label = if state.terminal.active_tab == Some(tab.session_id) {
            format!("* {}", tab.title)
        } else {
            tab.title.clone()
        };

        tab_row = tab_row.push(button(text(label)).on_press(Message::ActivateTerminalTab {
            session_id: tab.session_id,
        }));
    }

    tabs.push(tab_row).into()
}

fn active_terminal_output(state: &AppState) -> Element<'_, Message> {
    let Some(session_id) = state.terminal.active_tab else {
        return column![text("Output").size(18), text("No active terminal tab.")].into();
    };

    let Some(tab) = state
        .terminal
        .tabs
        .iter()
        .find(|tab| tab.session_id == session_id)
    else {
        return column![
            text("Output").size(18),
            text("Active terminal tab missing.")
        ]
        .into();
    };

    let mut output = column![
        text("Output").size(18),
        text(format!(
            "{} | size: {}x{} | buffered lines: {}",
            tab.title,
            tab.size.columns,
            tab.size.rows,
            tab.buffer.len()
        )),
    ]
    .spacing(8);

    if tab.buffer.is_empty() {
        output = output.push(text("No output yet."));
    } else {
        for line in tab
            .buffer
            .iter()
            .rev()
            .take(OUTPUT_PREVIEW_LINES)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
        {
            output = output.push(text(line).width(Length::Fill));
        }
    }

    output = output.push(terminal_input_panel(state, tab));

    output.into()
}

fn terminal_input_panel<'a>(
    state: &'a AppState,
    tab: &'a crate::terminal::TerminalTabState,
) -> Element<'a, Message> {
    match state
        .sessions
        .tabs
        .iter()
        .find(|session| session.id == tab.session_id)
    {
        Some(session) if matches!(session.kind, SessionKind::Shell) => {
            shell_input_panel(state, tab)
        }
        Some(_) => text("Interactive input is available only on shell tabs.").into(),
        None => text("Active session metadata missing.").into(),
    }
}

fn shell_input_panel<'a>(
    state: &'a AppState,
    tab: &'a crate::terminal::TerminalTabState,
) -> Element<'a, Message> {
    let session_id = tab.session_id;
    let draft = state.ui.terminal_input_for(session_id);

    column![
        text("Shell input").size(18),
        row![
            text_input("enter command", draft)
                .on_input(move |input| Message::UpdateTerminalInputDraft { session_id, input })
                .on_submit(Message::SendTerminalInput { session_id })
                .width(Length::Fill),
            button("Send").on_press(Message::SendTerminalInput { session_id }),
        ]
        .spacing(8),
    ]
    .spacing(8)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::TerminalTabState;
    use uuid::Uuid;

    #[test]
    fn terminal_workspace_view_accepts_empty_state() {
        let state = AppState::default();

        let _element = view(&state);
    }

    #[test]
    fn terminal_workspace_view_accepts_populated_state() {
        let mut state = AppState::default();
        let session_id = crate::model::SessionId(Uuid::new_v4());
        let mut tab = TerminalTabState::new(session_id, "production");
        tab.buffer.push("starting ssh".to_owned());
        tab.buffer.push("ready".to_owned());
        state.terminal.open_tab(tab);

        let _element = view(&state);
    }
}
