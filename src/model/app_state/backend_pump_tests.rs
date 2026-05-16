use super::*;
use crate::backend::{
    BackendCommandKind, BackendEvent, BackendExecutionError, BackendExecutor, NoopBackendExecutor,
    ScriptedBackendExecutor, ScriptedBackendResponse,
};
use crate::model::{
    AuthProfile, Host, LOCAL_TERMINAL_SESSION_ID, SessionStatus, TransferStatus, TunnelKind,
    TunnelRule, TunnelStatus,
};

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

fn sample_tunnel_rule() -> TunnelRule {
    TunnelRule {
        name: "local-db".to_owned(),
        kind: TunnelKind::Local,
        bind_host: "127.0.0.1".to_owned(),
        bind_port: 15432,
        target_host: "10.0.0.5".to_owned(),
        target_port: 5432,
        auto_start: false,
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
fn backend_queue_pump_discards_failed_session_tail_commands() {
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
    assert_eq!(state.ui.last_error.as_deref(), outcome.error.as_deref());
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        &state.sessions.tabs[0].status,
        SessionStatus::Failed { reason } if reason.contains("不支持")
    ));
}

#[test]
fn backend_queue_pump_keeps_other_session_commands_after_terminal_error() {
    let mut state = AppState::default();
    let first = sample_host();
    let second = sample_host();
    let first_id = first.id;
    let second_id = second.id;
    state.storage.upsert_host(first);
    state.storage.upsert_host(second);
    state.apply(Message::OpenShell { host_id: first_id });
    state.apply(Message::OpenShell { host_id: second_id });
    let failed_session_id = state.sessions.tabs[0].id;
    let remaining_session_id = state.sessions.tabs[1].id;
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 1);
    assert_eq!(state.backend_commands.pending_count(), 2);
    assert!(
        state
            .backend_commands
            .iter()
            .all(|command| command.session_id() == remaining_session_id)
    );
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Failed { .. }
    ));
    assert!(matches!(
        state.sessions.tabs[1].status,
        SessionStatus::Connecting
    ));
    assert_ne!(failed_session_id, remaining_session_id);
}

#[test]
fn backend_queue_pump_marks_pruned_sftp_transfer_failed_on_terminal_error() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });
    state.apply(Message::UploadSftp { host_id });
    let transfer_id = state.sessions.transfers[0].id;
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 2);
    assert!(state.backend_commands.is_empty());
    assert_eq!(state.sessions.transfers[0].id, transfer_id);
    assert!(matches!(
        &state.sessions.transfers[0].status,
        TransferStatus::Failed { reason } if reason.contains("不支持")
    ));
    assert!(!state.sessions.sftp_browsers[0].loading);
}

#[test]
fn backend_queue_pump_marks_sftp_transfer_failed_on_executor_error() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::RemoteName,
        value: "app.tar.gz".to_owned(),
    });
    state.apply(Message::UploadSftp { host_id });
    let transfer_id = state.sessions.transfers[0].id;
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 2);
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        &state.sessions.transfers[0].status,
        TransferStatus::Failed { reason } if reason.contains("不支持")
    ));
    assert_eq!(state.sessions.transfers[0].id, transfer_id);
    assert!(!state.sessions.sftp_browsers[0].loading);
    assert!(
        state.sessions.sftp_browsers[0]
            .last_error
            .as_deref()
            .unwrap_or("")
            .contains("不支持")
    );
}

#[test]
fn backend_queue_pump_keeps_sftp_session_connected_on_sftp_operation_error() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();
    state
        .sessions
        .set_status(state.sessions.tabs[0].id, SessionStatus::Connected);
    state.apply(Message::RefreshSftp { host_id });
    let mut executor = FailingSftpExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 1);
    assert!(outcome.error.as_deref().unwrap_or("").contains("SFTP"));
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Connected
    ));
    assert!(!state.sessions.sftp_browsers[0].loading);
    assert!(
        state.sessions.sftp_browsers[0]
            .last_error
            .as_deref()
            .unwrap_or("")
            .contains("permission denied")
    );
}

#[test]
fn backend_queue_pump_marks_tunnel_failed_on_executor_error() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::StartTunnel {
        host_id,
        rule: sample_tunnel_rule(),
    });
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 1);
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        &state.sessions.tabs[0].status,
        SessionStatus::Failed { reason } if reason.contains("不支持")
    ));
    assert!(matches!(
        state.sessions.tunnels[0].status,
        TunnelStatus::Failed
    ));
    assert!(
        state.sessions.tunnels[0]
            .last_error
            .as_deref()
            .unwrap_or("")
            .contains("不支持")
    );
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

#[test]
fn local_terminal_input_reaches_backend_and_updates_terminal_buffer() {
    let mut state = AppState::default();
    let session_id = LOCAL_TERMINAL_SESSION_ID;
    state.apply(Message::UpdateTerminalInputDraft {
        session_id,
        input: "echo smagicalssh-visible".to_owned(),
    });
    state.apply(Message::SendTerminalInput { session_id });

    let mut executor = ScriptedBackendExecutor::new();
    executor.push_response(ScriptedBackendResponse::new(
        BackendCommandKind::SendShellInput,
        vec![BackendEvent::Output {
            session_id,
            line: "smagicalssh-visible".to_owned(),
        }],
    ));

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(state.ui.terminal_input_for(session_id), "");
    assert_eq!(
        state
            .terminal
            .tabs
            .iter()
            .find(|tab| tab.session_id == session_id)
            .map(|tab| tab.buffer.as_slice()),
        Some(
            [
                format!(
                    "{} echo smagicalssh-visible",
                    crate::backend::LocalShellProfile::default_for_platform().prompt
                ),
                "smagicalssh-visible".to_owned(),
            ]
            .as_slice()
        )
    );
}

#[test]
fn local_terminal_send_clears_input_immediately_without_waiting_for_pump() {
    let mut state = AppState::default();
    let session_id = LOCAL_TERMINAL_SESSION_ID;

    state.apply(Message::UpdateTerminalInputDraft {
        session_id,
        input: "pwd".to_owned(),
    });
    let outcome = state.apply(Message::SendTerminalInput { session_id });

    assert!(outcome.changed());
    assert_eq!(state.ui.terminal_input_for(session_id), "");
    assert_eq!(state.backend_commands.pending_count(), 1);
}

#[derive(Debug, Clone, Copy)]
struct FailingSftpExecutor;

impl BackendExecutor for FailingSftpExecutor {
    fn execute(
        &mut self,
        command: crate::backend::BackendCommand,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        assert!(matches!(
            command,
            crate::backend::BackendCommand::Sftp { .. }
        ));

        Err(BackendExecutionError::SftpFailed {
            operation: "list dir".to_owned(),
            reason: "permission denied".to_owned(),
        })
    }
}
