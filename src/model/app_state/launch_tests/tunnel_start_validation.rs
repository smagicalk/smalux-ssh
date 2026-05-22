use super::*;

#[test]
fn start_tunnel_rejects_invalid_rule_without_side_effects() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    let mut rule = tunnel_rule(TunnelKind::Local);
    rule.name.clear();

    let outcome = state.apply(Message::StartTunnel { host_id, rule });

    assert!(
        outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("隧道规则无效")
    );
    assert_eq!(state.sessions.tab_count(), 0);
    assert!(state.backend_commands.is_empty());
}
