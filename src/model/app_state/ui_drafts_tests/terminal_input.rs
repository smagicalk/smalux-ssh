use super::*;

#[test]
fn terminal_input_draft_message_updates_ui_state_only() {
    let mut state = AppState::default();
    let session_id = SessionId(Uuid::new_v4());

    let outcome = state.apply(Message::UpdateTerminalInputDraft {
        session_id,
        input: "ls".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(state.ui.terminal_input_for(session_id), "ls");
    assert_eq!(state.backend_commands.pending_count(), 0);
}

#[test]
fn terminal_key_messages_edit_input_draft_without_backend_side_effects() {
    let mut state = AppState::default();
    let session_id = SessionId(Uuid::new_v4());

    state.apply(Message::AppendTerminalInputDraft {
        session_id,
        text: "ls".to_owned(),
    });
    state.apply(Message::AppendTerminalInputDraft {
        session_id,
        text: "\u{e001}".to_owned(),
    });
    state.apply(Message::BackspaceTerminalInputDraft { session_id });

    assert_eq!(state.ui.terminal_input_for(session_id), "l");
    assert_eq!(state.backend_commands.pending_count(), 0);
}

#[test]
fn local_terminal_input_is_visible_and_queues_on_enter() {
    let mut state = AppState::default();
    let session_id = crate::model::LOCAL_TERMINAL_SESSION_ID;

    let text = state.apply(Message::UpdateTerminalInputDraft {
        session_id,
        input: "echo smagicalssh-local".to_owned(),
    });
    assert!(text.changed());
    assert_eq!(
        state.ui.terminal_input_for(session_id),
        "echo smagicalssh-local"
    );

    let enter = state.apply(Message::SendTerminalInput { session_id });

    assert!(enter.changed());
    assert_eq!(state.backend_commands.pending_count(), 1);
    assert_eq!(state.ui.terminal_input_for(session_id), "");
    assert_eq!(
        state.terminal.tabs[0].buffer,
        vec![format!(
            "{} echo smagicalssh-local",
            crate::backend::LocalShellProfile::default_for_platform().prompt
        )]
    );
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::SendShellInput { session_id: queued_session_id, input })
            if *queued_session_id == session_id && input == "echo smagicalssh-local\n"
    ));
    assert_eq!(state.storage.command_history_count(), 1);
    assert_eq!(state.storage.command_history[0].host_id, None);
}

#[test]
fn local_terminal_starts_without_help_banner() {
    let mut state = AppState::default();
    let session_id = crate::model::LOCAL_TERMINAL_SESSION_ID;

    assert!(ui_drafts::ensure_local_terminal_tab(&mut state, session_id));
    assert!(!ui_drafts::ensure_local_terminal_tab(
        &mut state, session_id
    ));

    let tab = state
        .terminal
        .tabs
        .iter()
        .find(|tab| tab.session_id == session_id)
        .expect("local terminal tab should exist");
    assert_eq!(tab.title, crate::model::DEFAULT_LOCAL_TERMINAL_TITLE);
    assert!(tab.buffer.is_empty());
}

#[test]
fn local_terminal_empty_enter_queues_newline_without_history() {
    let mut state = AppState::default();
    let session_id = crate::model::LOCAL_TERMINAL_SESSION_ID;

    ui_drafts::ensure_local_terminal_tab(&mut state, session_id);
    state.apply(Message::UpdateTerminalInputDraft {
        session_id,
        input: String::new(),
    });

    let outcome = state.apply(Message::SendTerminalInput { session_id });

    assert!(outcome.changed());
    assert_eq!(state.ui.terminal_input_for(session_id), "");
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::SendShellInput { session_id: queued_session_id, input })
            if *queued_session_id == session_id && input == "\n"
    ));
    assert_eq!(state.storage.command_history_count(), 0);
}
