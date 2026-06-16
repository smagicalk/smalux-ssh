use super::*;

#[test]
fn close_session_tab_message_reports_missing_tab() {
    let mut state = desktop_state();
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());

    let outcome = state.apply_message(Message::CloseSessionTab { session_id });

    assert!(outcome.state_changed);
    assert!(outcome.error.is_some());
    assert_eq!(state.ui.last_error.as_deref(), outcome.error.as_deref());
    assert!(state.core.backend_commands.is_empty());

    let dismiss_outcome = state.apply_message(Message::DismissUiError);

    assert!(dismiss_outcome.changed());
    assert!(state.ui.last_error.is_none());
}
