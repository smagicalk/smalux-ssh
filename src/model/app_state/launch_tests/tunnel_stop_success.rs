use super::*;

#[test]
fn stop_tunnel_message_marks_runtime_stopping_before_backend_ack() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    let rule = tunnel_rule(TunnelKind::Local);
    state.core.storage.upsert_host(host);
    state.apply_message(Message::StartTunnel {
        host_id,
        rule: rule.clone(),
    });
    let session_id = state.core.sessions.tabs[0].id;
    state.core.backend_commands.drain();

    let outcome = state.apply_message(Message::StopTunnel {
        session_id,
        rule_name: rule.name,
    });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert!(matches!(
        state.core.sessions.tunnels[0].status,
        TunnelStatus::Stopping
    ));
    assert!(matches!(
        state.core.backend_commands.drain().as_slice(),
        [BackendCommand::StopTunnel {
            session_id: queued_session_id,
            request: TunnelStopRequest { rule_name },
        }] if *queued_session_id == session_id && rule_name == "local-db"
    ));
}

#[test]
fn stop_tunnel_message_normalizes_rule_name() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    let rule = tunnel_rule(TunnelKind::Local);
    state.core.storage.upsert_host(host);
    state.apply_message(Message::StartTunnel {
        host_id,
        rule: rule.clone(),
    });
    let session_id = state.core.sessions.tabs[0].id;
    state.core.backend_commands.drain();

    let outcome = state.apply_message(Message::StopTunnel {
        session_id,
        rule_name: " local-db ".to_owned(),
    });

    assert_eq!(outcome.queued_backend_commands, 1);
    assert!(matches!(
        state.core.sessions.tunnels[0].status,
        TunnelStatus::Stopping
    ));
    assert!(matches!(
        state.core.backend_commands.drain().as_slice(),
        [BackendCommand::StopTunnel {
            request: TunnelStopRequest { rule_name },
            ..
        }] if rule_name == "local-db"
    ));
}
