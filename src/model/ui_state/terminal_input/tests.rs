use super::super::UiState;
use super::TerminalInputDraft;
use crate::model::SessionId;
use uuid::Uuid;

#[test]
fn terminal_input_draft_starts_empty() {
    let draft = TerminalInputDraft::new(SessionId(Uuid::new_v4()));

    assert!(draft.input.is_empty());
}

#[test]
fn ui_state_terminal_input_messages_update_draft_only() {
    let mut ui = UiState::default();
    let first = SessionId(Uuid::new_v4());
    let second = SessionId(Uuid::new_v4());

    ui.set_terminal_input(first, "ls");
    ui.set_terminal_input(second, "pwd");
    ui.append_terminal_input(first, " -la");

    assert_eq!(ui.terminal_input_for(first), "ls -la");
    assert_eq!(ui.terminal_input_for(second), "pwd");
    assert_eq!(ui.terminal_input_for(SessionId(Uuid::new_v4())), "");

    ui.backspace_terminal_input(first);
    assert_eq!(ui.terminal_input_for(first), "ls -l");

    ui.clear_terminal_input(first);
    assert_eq!(ui.terminal_input_for(first), "");
    assert_eq!(ui.terminal_input_for(second), "pwd");
}
