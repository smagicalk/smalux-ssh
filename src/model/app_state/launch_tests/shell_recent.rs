use super::*;

#[test]
fn open_recent_connection_reuses_shell_launch_flow() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state
        .storage
        .record_recent_connection(crate::model::RecentConnection {
            host_id,
            label: "production".to_owned(),
            connected_at_unix_secs: 1,
        });

    let outcome = state.apply(Message::OpenRecentConnection { host_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 2);
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.terminal.tab_count(), 1);
    assert_eq!(state.ui.workspace.active_page, WorkspacePage::Terminal);
    assert_eq!(state.storage.recent_count(), 1);
    assert_eq!(state.storage.recent_connections[0].label, "production");
    assert_eq!(state.backend_commands.pending_count(), 2);
}

#[test]
fn open_recent_connection_reports_deleted_host() {
    let mut state = AppState::default();
    let host_id = HostId(uuid::Uuid::new_v4());

    let outcome = state.apply(Message::OpenRecentConnection { host_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert_eq!(state.ui.last_error.as_deref(), outcome.error.as_deref());
    assert_eq!(state.sessions.tab_count(), 0);
    assert!(state.backend_commands.is_empty());
}
