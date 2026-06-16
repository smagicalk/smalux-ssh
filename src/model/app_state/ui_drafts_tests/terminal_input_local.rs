use super::*;

#[test]
fn local_terminal_input_is_visible_and_queues_on_enter() {
    let mut state = desktop_state();
    state.apply_message(Message::OpenLocalTerminal);
    let session_id = state
        .core
        .sessions
        .active_tab
        .expect("local terminal should open");
    state.core.backend_commands.drain();

    let text = state.apply_message(Message::UpdateTerminalInputDraft {
        session_id,
        input: "echo smagicalssh-local".to_owned(),
    });
    assert!(text.changed());
    assert_eq!(
        state.ui.terminal_input_for(session_id),
        "echo smagicalssh-local"
    );

    let enter = state.apply_message(Message::SendTerminalInput { session_id });

    assert!(enter.changed());
    assert_eq!(state.core.backend_commands.pending_count(), 1);
    assert_eq!(state.ui.terminal_input_for(session_id), "");
    assert_eq!(
        state.core.terminal.tabs[0].buffer,
        vec![format!(
            "{} echo smagicalssh-local",
            crate::backend::LocalShellProfile::default_for_platform().prompt
        )]
    );
    assert!(matches!(
        state.core.backend_commands.front(),
        Some(BackendCommand::SendShellInput { session_id: queued_session_id, input })
            if *queued_session_id == session_id && input == "echo smagicalssh-local\n"
    ));
    assert_eq!(state.core.storage.command_history_count(), 1);
    assert_eq!(state.core.storage.command_history[0].host_id, None);
}

#[test]
fn local_terminal_starts_without_help_banner() {
    let mut state = desktop_state();
    state.apply_message(Message::OpenLocalTerminal);
    let session_id = state
        .core
        .sessions
        .active_tab
        .expect("local terminal should open");

    let tab = state
        .core
        .terminal
        .tabs
        .iter()
        .find(|tab| tab.session_id == session_id)
        .expect("local terminal tab should exist");
    assert_eq!(tab.title, crate::model::DEFAULT_LOCAL_TERMINAL_TITLE);
    assert!(tab.buffer.is_empty());
}

#[test]
fn open_local_terminal_message_creates_and_activates_new_tab_each_time() {
    let mut state = desktop_state();

    let first = state.apply_message(Message::OpenLocalTerminal);
    let first_session_id = state
        .core
        .sessions
        .active_tab
        .expect("first tab should be active");
    let second = state.apply_message(Message::OpenLocalTerminal);
    let second_session_id = state
        .core
        .sessions
        .active_tab
        .expect("second tab should be active");

    assert!(first.changed());
    assert!(second.changed());
    assert_ne!(first_session_id, second_session_id);
    assert_eq!(state.core.sessions.tab_count(), 2);
    assert_eq!(state.core.terminal.tab_count(), 2);
    assert_eq!(
        state.core.sessions.tabs[0].title,
        crate::model::DEFAULT_LOCAL_TERMINAL_TITLE
    );
    assert_eq!(state.core.sessions.tabs[1].title, "Local Shell 2");
    assert_eq!(
        state.core.terminal.tabs[0].title,
        crate::model::DEFAULT_LOCAL_TERMINAL_TITLE
    );
    assert_eq!(state.core.terminal.tabs[1].title, "Local Shell 2");
    assert_eq!(state.core.sessions.active_tab, Some(second_session_id));
    assert_eq!(state.core.terminal.active_tab, Some(second_session_id));
    assert_eq!(
        state.ui.workspace.active_page,
        crate::model::WorkspacePage::Terminal
    );
    assert!(matches!(
        state.core.backend_commands.drain().as_slice(),
        [
            BackendCommand::OpenLocalShell {
                session_id: first_queued,
                ..
            },
            BackendCommand::OpenLocalShell {
                session_id: second_queued,
                ..
            }
        ] if *first_queued == first_session_id && *second_queued == second_session_id
    ));
}

#[test]
fn open_local_terminal_reuses_first_available_title_number() {
    let mut state = desktop_state();
    state.apply_message(Message::OpenLocalTerminal);
    state.apply_message(Message::OpenLocalTerminal);
    state.apply_message(Message::OpenLocalTerminal);
    let second_session_id = state.core.sessions.tabs[1].id;

    state.apply_message(Message::CloseSessionTab {
        session_id: second_session_id,
    });
    state.apply_message(Message::OpenLocalTerminal);

    let titles: Vec<&str> = state
        .core
        .sessions
        .tabs
        .iter()
        .map(|tab| tab.title.as_str())
        .collect();
    assert_eq!(
        titles,
        vec![
            crate::model::DEFAULT_LOCAL_TERMINAL_TITLE,
            "Local Shell 3",
            "Local Shell 2"
        ]
    );
}

#[test]
fn new_local_terminal_input_queues_to_its_own_session() {
    let mut state = desktop_state();
    state.apply_message(Message::OpenLocalTerminal);
    let session_id = state
        .core
        .sessions
        .active_tab
        .expect("local terminal should be active");
    state.core.backend_commands.drain();

    state.apply_message(Message::UpdateTerminalInputDraft {
        session_id,
        input: "echo second-local".to_owned(),
    });
    let outcome = state.apply_message(Message::SendTerminalInput { session_id });

    assert!(outcome.changed());
    assert!(matches!(
        state.core.backend_commands.front(),
        Some(BackendCommand::SendShellInput { session_id: queued_session_id, input })
            if *queued_session_id == session_id && input == "echo second-local\n"
    ));
    assert_eq!(
        state.core.terminal.tabs[0].buffer,
        vec![format!(
            "{} echo second-local",
            crate::backend::LocalShellProfile::default_for_platform().prompt
        )]
    );
}

#[test]
fn local_terminal_empty_enter_queues_newline_without_history() {
    let mut state = desktop_state();
    state.apply_message(Message::OpenLocalTerminal);
    let session_id = state
        .core
        .sessions
        .active_tab
        .expect("local terminal should open");
    state.core.backend_commands.drain();

    state.apply_message(Message::UpdateTerminalInputDraft {
        session_id,
        input: String::new(),
    });

    let outcome = state.apply_message(Message::SendTerminalInput { session_id });

    assert!(outcome.changed());
    assert_eq!(state.ui.terminal_input_for(session_id), "");
    assert!(matches!(
        state.core.backend_commands.front(),
        Some(BackendCommand::SendShellInput { session_id: queued_session_id, input })
            if *queued_session_id == session_id && input == "\n"
    ));
    assert_eq!(state.core.storage.command_history_count(), 0);
}

#[test]
fn local_terminal_repeated_send_after_clear_does_not_duplicate_echo() {
    let mut state = desktop_state();
    state.apply_message(Message::OpenLocalTerminal);
    let session_id = state
        .core
        .sessions
        .active_tab
        .expect("local terminal should open");
    state.core.backend_commands.drain();

    state.apply_message(Message::UpdateTerminalInputDraft {
        session_id,
        input: "pwd".to_owned(),
    });
    state.apply_message(Message::SendTerminalInput { session_id });
    state.apply_message(Message::SendTerminalInput { session_id });

    assert_eq!(
        state.core.terminal.tabs[0].buffer,
        vec![format!(
            "{} pwd",
            crate::backend::LocalShellProfile::default_for_platform().prompt
        )]
    );
}
