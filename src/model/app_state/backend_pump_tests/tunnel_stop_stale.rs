use super::*;

#[test]
fn backend_queue_pump_skips_stale_tunnel_stop_commands_for_same_rule() {
    let mut state = CoreState::default();
    let host = sample_host();
    let host_id = host.id;
    let rule = sample_tunnel_rule();
    let stale_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let current_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.storage.upsert_host(host);
    state
        .sessions
        .open_tunnel_tab(current_session_id, host_id, &rule);
    state
        .sessions
        .set_status(current_session_id, SessionStatus::Connected);
    state
        .sessions
        .start_tunnel(current_session_id, &rule, Some(host_id), 20);
    state
        .sessions
        .mark_tunnel_running(current_session_id, &rule.name);
    state
        .backend_commands
        .push(crate::backend::BackendCommand::StopTunnel {
            session_id: stale_session_id,
            request: crate::backend::TunnelStopRequest::by_name(rule.name.clone()),
        });
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(!outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 0);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert_eq!(state.sessions.tunnels[0].session_id, current_session_id);
    assert!(matches!(
        state.sessions.tunnels[0].status,
        TunnelStatus::Running
    ));
}
