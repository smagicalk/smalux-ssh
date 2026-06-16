use super::*;

#[test]
fn start_tunnel_message_creates_tab_runtime_and_queues_backend_commands() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    state.core.storage.upsert_host(host);

    let outcome = state.apply_message(Message::StartTunnel {
        host_id,
        rule: tunnel_rule(TunnelKind::Local),
    });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 2);
    assert_eq!(state.core.sessions.tab_count(), 1);
    assert_eq!(state.core.sessions.tunnel_runtime_count(), 1);
    assert_eq!(state.ui.workspace.active_page, WorkspacePage::Tunnels);
    assert!(matches!(
        state.core.sessions.tabs[0].kind,
        SessionKind::Tunnel { .. }
    ));
    assert!(matches!(
        state.core.backend_commands.drain().as_slice(),
        [
            BackendCommand::Connect { .. },
            BackendCommand::StartTunnel { request, .. },
        ] if request.rule.name == "local-db"
    ));
}
