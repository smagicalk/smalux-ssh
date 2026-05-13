use super::*;
use crate::backend::{
    BackendCommandKind, BackendEvent, NoopBackendExecutor, ScriptedBackendExecutor,
    ScriptedBackendResponse,
};
use crate::model::{AuthProfile, Host, SessionStatus};

fn sample_host() -> Host {
    Host {
        id: HostId(uuid::Uuid::new_v4()),
        name: "production".to_owned(),
        group_id: None,
        tags: vec!["prod".to_owned()],
        address: "example.com".to_owned(),
        port: 22,
        auth: AuthProfile::Agent {
            username: "deploy".to_owned(),
            key_hint: Some("id_ed25519".to_owned()),
        },
        proxy: None,
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    }
}

#[test]
fn backend_queue_pump_executes_commands_and_applies_events() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenShell { host_id });
    let session_id = state.sessions.tabs[0].id;
    let mut executor = ScriptedBackendExecutor::new();
    executor.push_response(ScriptedBackendResponse::new(
        BackendCommandKind::Connect,
        vec![BackendEvent::Connected { session_id }],
    ));
    executor.push_response(ScriptedBackendResponse::new(
        BackendCommandKind::OpenShell,
        vec![BackendEvent::Output {
            session_id,
            line: "ready".to_owned(),
        }],
    ));

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 2);
    assert_eq!(outcome.applied_backend_events, 2);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert_eq!(
        executor.executed(),
        &[BackendCommandKind::Connect, BackendCommandKind::OpenShell]
    );
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Connected
    ));
    assert_eq!(state.terminal.tabs[0].buffer, vec!["ready"]);
}

#[test]
fn backend_queue_pump_stops_on_executor_error_and_keeps_remaining_commands() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenShell { host_id });
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 1);
    assert!(outcome.error.as_deref().unwrap_or("").contains("不支持"));
    assert_eq!(state.backend_commands.pending_count(), 1);
    assert!(matches!(
        &state.sessions.tabs[0].status,
        SessionStatus::Failed { reason } if reason.contains("不支持")
    ));
}

#[test]
fn backend_queue_pump_noops_when_queue_is_empty() {
    let mut state = AppState::default();
    let mut executor = ScriptedBackendExecutor::new();

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(!outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 0);
    assert!(outcome.error.is_none());
}
