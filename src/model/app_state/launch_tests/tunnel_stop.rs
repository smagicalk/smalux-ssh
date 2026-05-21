use super::*;

#[test]
fn stop_tunnel_message_reports_missing_runtime_without_queueing_command() {
    let mut state = AppState::default();
    let session_id = SessionId(uuid::Uuid::new_v4());

    let outcome = state.apply(Message::StopTunnel {
        session_id,
        rule_name: "local-db".to_owned(),
    });

    assert!(outcome.changed());
    assert!(
        outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("没有可停止的运行态")
    );
    assert_eq!(outcome.queued_backend_commands, 0);
    assert!(state.backend_commands.is_empty());
}

#[test]
fn stop_tunnel_message_rejects_mismatched_session_without_queueing_command() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let rule = tunnel_rule(TunnelKind::Local);
    state.storage.upsert_host(host);
    state.apply(Message::StartTunnel {
        host_id,
        rule: rule.clone(),
    });
    let mut another_rule = tunnel_rule(TunnelKind::Local);
    another_rule.name = "metrics".to_owned();
    state.apply(Message::StartTunnel {
        host_id,
        rule: another_rule,
    });
    state.backend_commands.drain();
    let wrong_session_id = state.sessions.tabs[1].id;

    let outcome = state.apply(Message::StopTunnel {
        session_id: wrong_session_id,
        rule_name: rule.name,
    });

    assert!(outcome.changed());
    assert!(outcome.error.as_deref().unwrap_or("").contains("不匹配"));
    assert_eq!(outcome.queued_backend_commands, 0);
    assert_eq!(
        state.sessions.tunnel_status("local-db"),
        Some(&TunnelStatus::Starting)
    );
    assert!(state.backend_commands.is_empty());
}

#[test]
fn stop_tunnel_message_reports_missing_session_runtime_without_panic() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let rule = tunnel_rule(TunnelKind::Local);
    state.storage.upsert_host(host);
    state.apply(Message::StartTunnel {
        host_id,
        rule: rule.clone(),
    });
    let session_id = state.sessions.tabs[0].id;
    let other_session_id = SessionId(uuid::Uuid::new_v4());
    state.sessions.tunnels[0].session_id = other_session_id;
    state.backend_commands.drain();

    let outcome = state.apply(Message::StopTunnel {
        session_id,
        rule_name: rule.name,
    });

    assert!(outcome.changed());
    assert!(
        outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("没有当前会话的运行态")
    );
    assert_eq!(outcome.queued_backend_commands, 0);
    assert_eq!(state.sessions.tunnels[0].session_id, other_session_id);
    assert!(matches!(
        state.sessions.tunnels[0].status,
        TunnelStatus::Starting
    ));
    assert!(state.backend_commands.is_empty());
}

#[test]
fn stop_tunnel_message_marks_runtime_stopping_before_backend_ack() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let rule = tunnel_rule(TunnelKind::Local);
    state.storage.upsert_host(host);
    state.apply(Message::StartTunnel {
        host_id,
        rule: rule.clone(),
    });
    let session_id = state.sessions.tabs[0].id;
    state.backend_commands.drain();

    let outcome = state.apply(Message::StopTunnel {
        session_id,
        rule_name: rule.name,
    });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert!(matches!(
        state.sessions.tunnels[0].status,
        TunnelStatus::Stopping
    ));
    assert!(matches!(
        state.backend_commands.drain().as_slice(),
        [BackendCommand::StopTunnel {
            session_id: queued_session_id,
            request: TunnelStopRequest { rule_name },
        }] if *queued_session_id == session_id && rule_name == "local-db"
    ));
}

#[test]
fn stop_tunnel_message_normalizes_rule_name() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let rule = tunnel_rule(TunnelKind::Local);
    state.storage.upsert_host(host);
    state.apply(Message::StartTunnel {
        host_id,
        rule: rule.clone(),
    });
    let session_id = state.sessions.tabs[0].id;
    state.backend_commands.drain();

    let outcome = state.apply(Message::StopTunnel {
        session_id,
        rule_name: " local-db ".to_owned(),
    });

    assert_eq!(outcome.queued_backend_commands, 1);
    assert!(matches!(
        state.sessions.tunnels[0].status,
        TunnelStatus::Stopping
    ));
    assert!(matches!(
        state.backend_commands.drain().as_slice(),
        [BackendCommand::StopTunnel {
            request: TunnelStopRequest { rule_name },
            ..
        }] if rule_name == "local-db"
    ));
}

#[test]
fn stop_tunnel_message_rejects_empty_rule_name() {
    let mut state = AppState::default();
    let session_id = SessionId(uuid::Uuid::new_v4());

    let outcome = state.apply(Message::StopTunnel {
        session_id,
        rule_name: "  ".to_owned(),
    });

    assert!(outcome.changed());
    assert!(outcome.error.as_deref().unwrap_or("").contains("不能为空"));
    assert!(state.backend_commands.is_empty());
}

#[test]
fn stop_tunnel_message_rejects_duplicate_stop_without_queueing_command() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let rule = tunnel_rule(TunnelKind::Local);
    state.storage.upsert_host(host);
    state.apply(Message::StartTunnel {
        host_id,
        rule: rule.clone(),
    });
    let session_id = state.sessions.tabs[0].id;
    state.backend_commands.drain();

    let first = state.apply(Message::StopTunnel {
        session_id,
        rule_name: rule.name.clone(),
    });
    let duplicate = state.apply(Message::StopTunnel {
        session_id,
        rule_name: rule.name,
    });

    assert_eq!(first.queued_backend_commands, 1);
    assert!(duplicate.changed());
    assert!(
        duplicate
            .error
            .as_deref()
            .unwrap_or("")
            .contains("正在停止")
    );
    assert_eq!(duplicate.queued_backend_commands, 0);
    assert_eq!(state.backend_commands.pending_count(), 1);
}
