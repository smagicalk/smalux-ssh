use super::*;

#[test]
fn close_pending_tunnel_tab_removes_launch_commands_without_stop() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let rule = TunnelRule {
        name: "local-db".to_owned(),
        kind: TunnelKind::Local,
        bind_host: "127.0.0.1".to_owned(),
        bind_port: 15432,
        target_host: "10.0.0.5".to_owned(),
        target_port: 5432,
        auto_start: false,
    };
    state.storage.upsert_host(host);
    state.apply(Message::StartTunnel { host_id, rule });
    let session_id = state.sessions.tabs[0].id;
    assert_eq!(state.backend_commands.pending_count(), 2);
    assert!(matches!(
        state.sessions.tunnels[0].status,
        TunnelStatus::Starting
    ));

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert_eq!(outcome.queued_backend_commands, 0);
    assert_eq!(state.sessions.tab_count(), 0);
    assert_eq!(state.sessions.tunnel_runtime_count(), 0);
    assert!(state.backend_commands.is_empty());
}

#[test]
fn close_connected_pending_tunnel_tab_queues_disconnect_after_cancelling_launch() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let rule = TunnelRule {
        name: "local-db".to_owned(),
        kind: TunnelKind::Local,
        bind_host: "127.0.0.1".to_owned(),
        bind_port: 15432,
        target_host: "10.0.0.5".to_owned(),
        target_port: 5432,
        auto_start: false,
    };
    state.storage.upsert_host(host);
    state.apply(Message::StartTunnel { host_id, rule });
    let session_id = state.sessions.tabs[0].id;
    let connect_command = state
        .backend_commands
        .pop_front()
        .expect("连接命令应先入队");
    assert!(matches!(connect_command, BackendCommand::Connect { .. }));
    state
        .sessions
        .set_status(session_id, SessionStatus::Connected);
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::StartTunnel { .. })
    ));

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert_eq!(state.sessions.tab_count(), 0);
    assert_eq!(state.sessions.tunnel_runtime_count(), 0);
    assert_eq!(state.backend_commands.pending_count(), 1);
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::Disconnect {
            session_id: queued_session_id
        }) if *queued_session_id == session_id
    ));
}

#[test]
fn close_starting_tunnel_without_pending_launch_commands_requires_stop() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let rule = TunnelRule {
        name: "local-db".to_owned(),
        kind: TunnelKind::Local,
        bind_host: "127.0.0.1".to_owned(),
        bind_port: 15432,
        target_host: "10.0.0.5".to_owned(),
        target_port: 5432,
        auto_start: false,
    };
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
    let rule = TunnelRule {
        name: "local-db".to_owned(),
        kind: TunnelKind::Local,
        bind_host: "127.0.0.1".to_owned(),
        bind_port: 15432,
        target_host: "10.0.0.5".to_owned(),
        target_port: 5432,
        auto_start: false,
    };
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

#[test]
fn close_tunnel_tab_removes_only_matching_session_runtime() {
    let mut state = AppState::default();
    let closed_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let current_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    let rule = TunnelRule {
        name: "local-db".to_owned(),
        kind: TunnelKind::Local,
        bind_host: "127.0.0.1".to_owned(),
        bind_port: 15432,
        target_host: "10.0.0.5".to_owned(),
        target_port: 5432,
        auto_start: false,
    };
    state
        .sessions
        .open_tunnel_tab(closed_session_id, host_id, &rule);
    state.sessions.tunnels.push(TunnelRuntimeState {
        session_id: closed_session_id,
        rule_name: "local-db".to_owned(),
        host_id: Some(host_id),
        status: TunnelStatus::Stopped,
        started_at_unix_secs: None,
        last_error: None,
    });
    state.sessions.tunnels.push(TunnelRuntimeState {
        session_id: current_session_id,
        rule_name: "local-db".to_owned(),
        host_id: Some(host_id),
        status: TunnelStatus::Stopped,
        started_at_unix_secs: None,
        last_error: None,
    });

    let outcome = state.apply(Message::CloseSessionTab {
        session_id: closed_session_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.tunnel_runtime_count(), 1);
    assert_eq!(state.sessions.tunnels[0].session_id, current_session_id);
    assert!(matches!(
        state.sessions.tunnels[0].status,
        TunnelStatus::Stopped
    ));
}

#[test]
fn close_tunnel_tab_ignores_other_session_running_same_rule() {
    let mut state = AppState::default();
    let closed_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let current_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    let rule = TunnelRule {
        name: "local-db".to_owned(),
        kind: TunnelKind::Local,
        bind_host: "127.0.0.1".to_owned(),
        bind_port: 15432,
        target_host: "10.0.0.5".to_owned(),
        target_port: 5432,
        auto_start: false,
    };
    state
        .sessions
        .open_tunnel_tab(closed_session_id, host_id, &rule);
    state.sessions.tunnels.push(TunnelRuntimeState {
        session_id: closed_session_id,
        rule_name: "local-db".to_owned(),
        host_id: Some(host_id),
        status: TunnelStatus::Stopped,
        started_at_unix_secs: None,
        last_error: None,
    });
    state.sessions.tunnels.push(TunnelRuntimeState {
        session_id: current_session_id,
        rule_name: "local-db".to_owned(),
        host_id: Some(host_id),
        status: TunnelStatus::Running,
        started_at_unix_secs: Some(10),
        last_error: None,
    });

    let outcome = state.apply(Message::CloseSessionTab {
        session_id: closed_session_id,
    });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert_eq!(state.sessions.tunnel_runtime_count(), 1);
    assert_eq!(state.sessions.tunnels[0].session_id, current_session_id);
    assert!(matches!(
        state.sessions.tunnels[0].status,
        TunnelStatus::Running
    ));
}
