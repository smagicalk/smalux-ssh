use super::*;

#[test]
fn backend_queue_pump_skips_terminal_connect_commands() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.storage.upsert_host(host.clone());
    state
        .sessions
        .open_shell_tab(session_id, host_id, "production");
    state
        .sessions
        .set_status(session_id, SessionStatus::Disconnected);
    state
        .backend_commands
        .push(crate::backend::BackendCommand::Connect {
            session_id,
            target: crate::backend::ConnectionTarget::from_host(&host),
        });
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(!outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 1);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Disconnected
    ));
}

#[test]
fn backend_queue_pump_skips_mismatched_connect_commands() {
    let mut state = AppState::default();
    let host = sample_host();
    let mut stale_target_host = sample_host();
    stale_target_host.name = "stale".to_owned();
    let host_id = host.id;
    let stale_host_id = stale_target_host.id;
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
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 1);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert_ne!(host_id, stale_host_id);
    assert!(matches!(
        &state.sessions.tabs[0].status,
        SessionStatus::Failed { reason } if reason.contains("连接命令已失效")
    ));
}

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

#[test]
fn backend_queue_pump_keeps_other_session_tail_after_stale_connect_command() {
    let mut state = AppState::default();
    let host = sample_host();
    let mut stale_target_host = sample_host();
    stale_target_host.name = "stale".to_owned();
    let host_id = host.id;
    let stale_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let current_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.storage.upsert_host(host.clone());
    state.storage.upsert_host(stale_target_host.clone());
    state
        .sessions
        .open_shell_tab(stale_session_id, host_id, "stale");
    state
        .sessions
        .set_status(stale_session_id, SessionStatus::Connecting);
    state
        .sessions
        .open_shell_tab(current_session_id, host_id, "current");
    state
        .sessions
        .set_status(current_session_id, SessionStatus::Connecting);
    state
        .backend_commands
        .push(crate::backend::BackendCommand::Connect {
            session_id: stale_session_id,
            target: crate::backend::ConnectionTarget::from_host(&stale_target_host),
        });
    state
        .backend_commands
        .push(crate::backend::BackendCommand::OpenShell {
            session_id: stale_session_id,
            pty: crate::backend::PtyRequest::xterm(crate::terminal::TerminalSize::default()),
        });
    state
        .backend_commands
        .push(crate::backend::BackendCommand::OpenShell {
            session_id: current_session_id,
            pty: crate::backend::PtyRequest::xterm(crate::terminal::TerminalSize::default()),
        });
    let mut executor = ScriptedBackendExecutor::new();
    executor.push_response(ScriptedBackendResponse::new(
        BackendCommandKind::OpenShell,
        vec![BackendEvent::ShellOpened {
            session_id: current_session_id,
        }],
    ));

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 1);
    assert_eq!(outcome.applied_backend_events, 2);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert_eq!(executor.executed(), &[BackendCommandKind::OpenShell]);
    assert!(matches!(
        &state.sessions.tabs[0].status,
        SessionStatus::Failed { reason } if reason.contains("连接命令已失效")
    ));
    assert!(matches!(
        state.sessions.tabs[1].status,
        SessionStatus::Connected
    ));
}
