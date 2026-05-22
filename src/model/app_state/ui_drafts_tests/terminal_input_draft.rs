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
