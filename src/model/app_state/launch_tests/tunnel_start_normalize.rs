use super::*;

#[test]
fn start_tunnel_normalizes_rule_before_opening_runtime() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    let mut rule = tunnel_rule(TunnelKind::Local);
    rule.name = " local-db ".to_owned();
    rule.bind_host = " 127.0.0.1 ".to_owned();
    rule.target_host = " 10.0.0.5 ".to_owned();
    state.core.storage.upsert_host(host);

    let outcome = state.apply_message(Message::StartTunnel { host_id, rule });

    assert!(outcome.changed());
    assert_eq!(state.core.sessions.tunnels[0].rule_name, "local-db");
    assert_eq!(
        state.core.sessions.tabs[0].title,
        "L 127.0.0.1:15432 -> 10.0.0.5:5432"
    );
    assert!(matches!(
        &state.core.sessions.tabs[0].kind,
        SessionKind::Tunnel { rule_name } if rule_name == "local-db"
    ));
    assert!(matches!(
        state.core.backend_commands.drain().as_slice(),
        [
            BackendCommand::Connect { .. },
            BackendCommand::StartTunnel { request, .. },
        ] if request.rule.name == "local-db"
            && request.rule.bind_host == "127.0.0.1"
            && request.rule.target_host == "10.0.0.5"
    ));
}
