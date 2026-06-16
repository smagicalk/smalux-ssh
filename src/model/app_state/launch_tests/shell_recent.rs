use super::*;

#[test]
fn open_recent_connection_reports_deleted_host() {
    let mut state = desktop_state();
    let host_id = HostId(uuid::Uuid::new_v4());

    let outcome = state.apply_message(Message::OpenRecentConnection { host_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert_eq!(state.ui.last_error.as_deref(), outcome.error.as_deref());
    assert_eq!(state.core.sessions.tab_count(), 0);
    assert!(state.core.backend_commands.is_empty());
}
