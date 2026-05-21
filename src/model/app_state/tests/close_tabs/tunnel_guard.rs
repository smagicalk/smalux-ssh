use super::*;

#[test]
fn close_starting_tunnel_without_pending_launch_commands_requires_stop() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let rule = tunnel_rule();
    state.storage.upsert_host(host);
    state.apply(Message::StartTunnel { host_id, rule });
    let session_id = state.sessions.tabs[0].id;
    state.backend_commands.drain();

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.sessions.tunnel_runtime_count(), 1);
    assert!(state.backend_commands.is_empty());
}

#[test]
fn close_session_tab_message_requires_stopping_running_tunnel_first() {
    let mut state = AppState::default();
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    let rule = tunnel_rule();
    state.sessions.open_tunnel_tab(session_id, host_id, &rule);
    state.sessions.tunnels.push(TunnelRuntimeState {
        session_id,
        rule_name: "local-db".to_owned(),
        host_id: Some(host_id),
        status: TunnelStatus::Running,
        started_at_unix_secs: Some(1),
        last_error: None,
    });

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.state_changed);
    assert!(outcome.error.is_some());
    assert_eq!(state.ui.last_error.as_deref(), outcome.error.as_deref());
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.sessions.tunnel_runtime_count(), 1);
    assert!(state.backend_commands.is_empty());
}
