use super::*;

#[test]
fn backend_queue_pump_skips_terminal_tunnel_start_commands() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let rule = sample_tunnel_rule();
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.storage.upsert_host(host);
    state.sessions.open_tunnel_tab(session_id, host_id, &rule);
    state
        .sessions
        .set_status(session_id, SessionStatus::Connected);
    state
        .sessions
        .start_tunnel(session_id, &rule, Some(host_id), 10);
    state
        .sessions
        .fail_tunnel_for_session_rule(session_id, &rule.name, "connection lost");
    state
        .backend_commands
        .push(crate::backend::BackendCommand::StartTunnel {
            session_id,
            request: crate::backend::TunnelStartRequest::new(rule.clone())
                .expect("测试隧道规则应有效"),
        });
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(!outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 0);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        state.sessions.tunnels[0].status,
        TunnelStatus::Failed
    ));
    assert_eq!(
        state.sessions.tunnels[0].last_error.as_deref(),
        Some("connection lost")
    );
}

#[test]
fn backend_queue_pump_skips_tunnel_start_when_session_is_terminal() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let rule = sample_tunnel_rule();
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.storage.upsert_host(host);
    state.sessions.open_tunnel_tab(session_id, host_id, &rule);
    state
        .sessions
        .set_status(session_id, SessionStatus::Disconnected);
    state
        .sessions
        .start_tunnel(session_id, &rule, Some(host_id), 10);
    state
        .backend_commands
        .push(crate::backend::BackendCommand::StartTunnel {
            session_id,
            request: crate::backend::TunnelStartRequest::new(rule.clone())
                .expect("测试隧道规则应有效"),
        });
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(!outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 0);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        state.sessions.tunnels[0].status,
        TunnelStatus::Starting
    ));
}
