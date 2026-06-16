use super::*;

#[test]
fn close_tunnel_tab_removes_only_matching_session_runtime() {
    let mut state = core_state();
    let closed_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let current_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    let rule = tunnel_rule();
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
    let mut state = core_state();
    let closed_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let current_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    let rule = tunnel_rule();
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
