use super::*;

#[test]
fn open_shell_message_reports_missing_host_without_queueing_commands() {
    let mut state = desktop_state();
    let host_id = HostId(uuid::Uuid::new_v4());

    let outcome = state.apply_message(Message::OpenShell { host_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert_eq!(state.core.sessions.tab_count(), 0);
    assert!(state.core.backend_commands.is_empty());
}
