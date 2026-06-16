use super::*;

#[test]
fn close_pending_tunnel_tab_removes_launch_commands_without_stop() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    let rule = tunnel_rule();
    state.core.storage.upsert_host(host);
    state.apply_message(Message::StartTunnel { host_id, rule });
    let session_id = state.core.sessions.tabs[0].id;
    assert_eq!(state.core.backend_commands.pending_count(), 2);
    assert!(matches!(
        state.core.sessions.tunnels[0].status,
        TunnelStatus::Starting
    ));

    let outcome = state.apply_message(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert_eq!(outcome.queued_backend_commands, 0);
    assert_eq!(state.core.sessions.tab_count(), 0);
    assert_eq!(state.core.sessions.tunnel_runtime_count(), 0);
    assert!(state.core.backend_commands.is_empty());
}

#[test]
fn close_connected_pending_tunnel_tab_queues_disconnect_after_cancelling_launch() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    let rule = tunnel_rule();
    state.core.storage.upsert_host(host);
    state.apply_message(Message::StartTunnel { host_id, rule });
    let session_id = state.core.sessions.tabs[0].id;
    let connect_command = state
        .core
        .backend_commands
        .pop_front()
        .expect("连接命令应先入队");
    assert!(matches!(connect_command, BackendCommand::Connect { .. }));
    state
        .core
        .sessions
        .set_status(session_id, SessionStatus::Connected);
    assert!(matches!(
        state.core.backend_commands.front(),
        Some(BackendCommand::StartTunnel { .. })
    ));

    let outcome = state.apply_message(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert_eq!(state.core.sessions.tab_count(), 0);
    assert_eq!(state.core.sessions.tunnel_runtime_count(), 0);
    assert_eq!(state.core.backend_commands.pending_count(), 1);
    assert!(matches!(
        state.core.backend_commands.front(),
        Some(BackendCommand::Disconnect {
            session_id: queued_session_id
        }) if *queued_session_id == session_id
    ));
}
