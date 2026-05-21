use super::*;

#[test]
fn backend_queue_pump_marks_tunnel_failed_on_executor_error() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::StartTunnel {
        host_id,
        rule: sample_tunnel_rule(),
    });
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 1);
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        &state.sessions.tabs[0].status,
        SessionStatus::Failed { reason } if reason.contains("不支持")
    ));
    assert!(matches!(
        state.sessions.tunnels[0].status,
        TunnelStatus::Failed
    ));
    assert!(
        state.sessions.tunnels[0]
            .last_error
            .as_deref()
            .unwrap_or("")
            .contains("不支持")
    );
}
