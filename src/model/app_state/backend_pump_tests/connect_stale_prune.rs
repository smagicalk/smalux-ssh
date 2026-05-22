use super::*;

#[test]
fn backend_queue_pump_prunes_tail_after_stale_connect_command() {
    let mut state = AppState::default();
    let host = sample_host();
    let mut stale_target_host = sample_host();
    stale_target_host.name = "stale".to_owned();
    let host_id = host.id;
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.storage.upsert_host(host);
    state.storage.upsert_host(stale_target_host.clone());
    state
        .sessions
        .open_shell_tab(session_id, host_id, "production");
    state
        .sessions
        .set_status(session_id, SessionStatus::Connecting);
    state
        .backend_commands
        .push(crate::backend::BackendCommand::Connect {
            session_id,
            target: crate::backend::ConnectionTarget::from_host(&stale_target_host),
        });
    state
        .backend_commands
        .push(crate::backend::BackendCommand::OpenShell {
            session_id,
            pty: crate::backend::PtyRequest::xterm(crate::terminal::TerminalSize::default()),
        });
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 1);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        &state.sessions.tabs[0].status,
        SessionStatus::Failed { reason } if reason.contains("连接命令已失效")
    ));
}
