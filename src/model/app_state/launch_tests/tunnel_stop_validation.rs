use super::*;

#[test]
fn stop_tunnel_message_rejects_empty_rule_name() {
    let mut state = CoreState::default();
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
    let mut state = CoreState::default();
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
