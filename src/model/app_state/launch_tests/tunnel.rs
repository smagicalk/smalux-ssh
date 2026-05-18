use super::*;
use crate::backend::{BackendCommand, TunnelStopRequest};
use crate::model::{SessionId, TunnelKind, TunnelRule, TunnelStatus};

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
fn start_tunnel_normalizes_rule_before_opening_runtime() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let mut rule = tunnel_rule(TunnelKind::Local);
    rule.name = " local-db ".to_owned();
    rule.bind_host = " 127.0.0.1 ".to_owned();
    rule.target_host = " 10.0.0.5 ".to_owned();
    state.storage.upsert_host(host);

    let outcome = state.apply(Message::StartTunnel { host_id, rule });

    assert!(outcome.changed());
    assert_eq!(state.sessions.tunnels[0].rule_name, "local-db");
    assert_eq!(
        state.sessions.tabs[0].title,
        "L 127.0.0.1:15432 -> 10.0.0.5:5432"
    );
    assert!(matches!(
        &state.sessions.tabs[0].kind,
        SessionKind::Tunnel { rule_name } if rule_name == "local-db"
    ));
    assert!(matches!(
        state.backend_commands.drain().as_slice(),
        [
            BackendCommand::Connect { .. },
            BackendCommand::StartTunnel { request, .. },
        ] if request.rule.name == "local-db"
            && request.rule.bind_host == "127.0.0.1"
            && request.rule.target_host == "10.0.0.5"
    ));
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
