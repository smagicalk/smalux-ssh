use super::*;

#[test]
fn close_session_tab_message_reports_missing_tab() {
    let mut state = AppState::default();
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.state_changed);
    assert!(outcome.error.is_some());
    assert_eq!(state.ui.last_error.as_deref(), outcome.error.as_deref());
    assert!(state.backend_commands.is_empty());

    let dismiss_outcome = state.apply(Message::DismissUiError);

    assert!(dismiss_outcome.changed());
    assert!(state.ui.last_error.is_none());
}
