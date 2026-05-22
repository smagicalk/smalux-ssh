use super::*;

#[test]
fn start_tunnel_rejects_duplicate_open_rule_without_queueing_commands() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let rule = tunnel_rule(TunnelKind::Local);
    state.storage.upsert_host(host);

    let first = state.apply(Message::StartTunnel {
        host_id,
        rule: rule.clone(),
    });
    let duplicate = state.apply(Message::StartTunnel { host_id, rule });

    assert!(first.changed());
    assert_eq!(first.queued_backend_commands, 2);
    assert!(
        duplicate
            .error
            .as_deref()
            .unwrap_or("")
            .contains("已有打开的标签页")
    );
    assert_eq!(duplicate.queued_backend_commands, 0);
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.sessions.tunnel_runtime_count(), 1);
    assert_eq!(state.backend_commands.pending_count(), 2);
}

#[test]
fn start_tunnel_rejects_duplicate_rule_after_normalization() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let mut duplicate = tunnel_rule(TunnelKind::Local);
    duplicate.name = " local-db ".to_owned();
    state.storage.upsert_host(host);

    let first = state.apply(Message::StartTunnel {
        host_id,
        rule: tunnel_rule(TunnelKind::Local),
    });
    let duplicate = state.apply(Message::StartTunnel {
        host_id,
        rule: duplicate,
    });

    assert_eq!(first.queued_backend_commands, 2);
    assert!(duplicate.changed());
    assert!(
        duplicate
            .error
            .as_deref()
            .unwrap_or("")
            .contains("已有打开的标签页")
    );
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.sessions.tunnel_runtime_count(), 1);
    assert_eq!(state.backend_commands.pending_count(), 2);
}
