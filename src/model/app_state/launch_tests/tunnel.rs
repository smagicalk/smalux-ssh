use super::*;
use crate::backend::{BackendCommand, TunnelStopRequest};
use crate::model::{SessionId, TunnelKind, TunnelRule};

fn tunnel_rule(kind: TunnelKind) -> TunnelRule {
    TunnelRule {
        name: "local-db".to_owned(),
        kind,
        bind_host: "127.0.0.1".to_owned(),
        bind_port: 15432,
        target_host: "10.0.0.5".to_owned(),
        target_port: 5432,
        auto_start: false,
    }
}

#[test]
fn start_tunnel_message_creates_tab_runtime_and_queues_backend_commands() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    let outcome = state.apply(Message::StartTunnel {
        host_id,
        rule: tunnel_rule(TunnelKind::Local),
    });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 2);
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.sessions.tunnel_runtime_count(), 1);
    assert_eq!(state.ui.workspace.active_page, WorkspacePage::Tunnels);
    assert!(matches!(
        state.sessions.tabs[0].kind,
        SessionKind::Tunnel { .. }
    ));
    assert!(matches!(
        state.backend_commands.drain().as_slice(),
        [
            BackendCommand::Connect { .. },
            BackendCommand::StartTunnel { request, .. },
        ] if request.rule.name == "local-db"
    ));
}

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

#[test]
fn stop_tunnel_message_queues_stop_command() {
    let mut state = AppState::default();
    let session_id = SessionId(uuid::Uuid::new_v4());

    let outcome = state.apply(Message::StopTunnel {
        session_id,
        rule_name: "local-db".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert!(matches!(
        state.backend_commands.drain().as_slice(),
        [BackendCommand::StopTunnel {
            session_id: queued_session_id,
            request: TunnelStopRequest { rule_name },
        }] if *queued_session_id == session_id && rule_name == "local-db"
    ));
}
