use super::*;
use crate::backend::{
    BackendCommandKind, BackendEvent, BackendExecutionError, BackendExecutor, NoopBackendExecutor,
    ScriptedBackendExecutor, ScriptedBackendResponse,
};
use crate::model::{
    AuthProfile, Host, HostKeyVerification, KeyAlgorithm, KnownHostEntry,
    LOCAL_TERMINAL_SESSION_ID, SessionStatus, TransferStatus, TunnelKind, TunnelRule, TunnelStatus,
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
fn backend_queue_pump_records_unknown_host_key_candidate() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenShell { host_id });
    let mut executor = RejectingHostKeyExecutor::new(HostKeyVerification::Unknown);

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.applied_backend_events, 1);
    assert!(state.backend_commands.is_empty());
    assert_eq!(state.storage.known_host_count(), 1);
    assert_eq!(
        state.storage.known_hosts[0],
        KnownHostEntry::untrusted("example.com", 22, KeyAlgorithm::Ed25519, "SHA256:new")
    );
    assert!(matches!(
        &state.sessions.tabs[0].status,
        SessionStatus::Failed { reason } if reason.contains("主机密钥未被信任")
    ));
}

#[test]
fn backend_queue_pump_does_not_overwrite_trusted_host_on_mismatch() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.storage.upsert_known_host(KnownHostEntry {
        host: "example.com".to_owned(),
        port: 22,
        key_algorithm: KeyAlgorithm::Ed25519,
        fingerprint: "SHA256:old".to_owned(),
        trusted: true,
    });
    state.apply(Message::OpenShell { host_id });
    let mut executor = RejectingHostKeyExecutor::new(HostKeyVerification::Mismatch {
        expected: "SHA256:old".to_owned(),
        actual: "SHA256:new".to_owned(),
    });

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(state.storage.known_host_count(), 1);
    assert_eq!(state.storage.known_hosts[0].fingerprint, "SHA256:old");
    assert!(state.storage.known_hosts[0].trusted);
    assert!(state.backend_commands.is_empty());
}

#[test]
fn backend_queue_pump_marks_failed_remote_command_history_finished() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::RunRemoteCommand {
        host_id,
        command: "uptime".to_owned(),
        request_pty: false,
    });
    state.storage.command_history[0].started_at_unix_secs = 1;
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 1);
    assert_eq!(state.storage.command_history[0].exit_code, None);
    assert!(state.storage.command_history[0].duration_ms.is_some());
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Failed { .. }
    ));
}

#[derive(Debug)]
struct RejectingHostKeyExecutor {
    verification: HostKeyVerification,
}

impl RejectingHostKeyExecutor {
    fn new(verification: HostKeyVerification) -> Self {
        Self { verification }
    }
}

impl BackendExecutor for RejectingHostKeyExecutor {
    fn execute(
        &mut self,
        _command: crate::backend::BackendCommand,
    ) -> Result<Vec<BackendEvent>, BackendExecutionError> {
        Err(BackendExecutionError::HostKeyRejected {
            host: "example.com".to_owned(),
            port: 22,
            key_algorithm: KeyAlgorithm::Ed25519,
            fingerprint: "SHA256:new".to_owned(),
            verification: self.verification.clone(),
        })
    }
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
    assert_eq!(outcome.applied_backend_events, 0);
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

    assert!(!outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 0);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert_ne!(host_id, stale_host_id);
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Connecting
    ));
}

#[test]
fn backend_queue_pump_skips_terminal_open_shell_commands() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.storage.upsert_host(host);
    state
        .sessions
        .open_shell_tab(session_id, host_id, "production");
    state
        .sessions
        .set_status(session_id, SessionStatus::Disconnected);
    state
        .backend_commands
        .push(crate::backend::BackendCommand::OpenShell {
            session_id,
            pty: crate::backend::PtyRequest::xterm(crate::terminal::TerminalSize::default()),
        });
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(!outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 0);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Disconnected
    ));
}

#[test]
fn backend_queue_pump_skips_terminal_remote_command_requests() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::RunRemoteCommand {
        host_id,
        command: "uptime".to_owned(),
        request_pty: false,
    });
    let session_id = state.sessions.tabs[0].id;
    state.backend_commands.drain();
    state
        .sessions
        .set_status(session_id, SessionStatus::Disconnected);
    state
        .backend_commands
        .push(crate::backend::BackendCommand::RunCommand {
            session_id,
            request: crate::backend::RemoteCommandRequest::exec("uptime"),
        });
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(!outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 0);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Disconnected
    ));
    assert_eq!(state.storage.command_history[0].exit_code, None);
}

#[test]
fn backend_queue_pump_skips_terminal_shell_drain_commands() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.storage.upsert_host(host);
    state
        .sessions
        .open_shell_tab(session_id, host_id, "production");
    state
        .sessions
        .set_status(session_id, SessionStatus::Connected);
    state
        .backend_commands
        .push(crate::backend::BackendCommand::DrainSessionOutput { session_id });
    state
        .sessions
        .set_status(session_id, SessionStatus::Disconnected);
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(!outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 0);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Disconnected
    ));
}

#[test]
fn backend_queue_pump_skips_terminal_shell_input_commands() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.storage.upsert_host(host);
    state
        .sessions
        .open_shell_tab(session_id, host_id, "production");
    state
        .sessions
        .set_status(session_id, SessionStatus::Connected);
    state
        .backend_commands
        .push(crate::backend::BackendCommand::SendShellInput {
            session_id,
            input: "uptime\n".to_owned(),
        });
    state
        .sessions
        .set_status(session_id, SessionStatus::Disconnected);
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(!outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 0);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Disconnected
    ));
}

#[test]
fn backend_queue_pump_skips_terminal_sftp_list_commands() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.storage.upsert_host(host);
    state
        .sessions
        .open_sftp_tab(session_id, host_id, "/home/ops");
    state
        .sessions
        .set_status(session_id, SessionStatus::Connected);
    state
        .sessions
        .set_sftp_loading_for_session(session_id, true);
    state
        .backend_commands
        .push(crate::backend::BackendCommand::Sftp {
            session_id,
            request: crate::backend::SftpRequest::ListDir {
                remote_path: "/home/ops".to_owned(),
            },
        });
    state
        .sessions
        .set_status(session_id, SessionStatus::Disconnected);
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 0);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert!(!state.sessions.sftp_browsers[0].loading);
    assert!(state.sessions.sftp_browsers[0].last_error.is_none());
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Disconnected
    ));
}

#[test]
fn backend_queue_pump_marks_terminal_sftp_transfers_failed_without_executor() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let session_id = state.sessions.tabs[0].id;
    state.backend_commands.drain();
    state
        .sessions
        .set_status(session_id, SessionStatus::Connected);
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
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/deploy.sh".to_owned(),
    });
    state.apply(Message::DownloadSftp {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });
    state
        .sessions
        .set_status(session_id, SessionStatus::Disconnected);
    let transfer_ids = state
        .sessions
        .transfers
        .iter()
        .map(|task| task.id)
        .collect::<Vec<_>>();
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 2);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert!(!state.sessions.sftp_browsers[0].loading);
    assert_eq!(
        state
            .sessions
            .transfers
            .iter()
            .map(|task| task.id)
            .collect::<Vec<_>>(),
        transfer_ids
    );
    assert!(state.sessions.transfers.iter().all(|task| matches!(
        &task.status,
        TransferStatus::Failed { reason } if reason.contains("SFTP 会话已结束")
    )));
}

#[test]
fn backend_queue_pump_marks_terminal_sftp_write_commands_failed_without_executor() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let session_id = state.sessions.tabs[0].id;
    state.backend_commands.drain();
    state
        .sessions
        .set_status(session_id, SessionStatus::Connected);
    state.apply(Message::RemoveSftpFile {
        host_id,
        remote_path: "/home/ops/old.log".to_owned(),
    });
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::NewDirName,
        value: "releases".to_owned(),
    });
    state.apply(Message::CreateSftpDir { host_id });
    state
        .sessions
        .set_status(session_id, SessionStatus::Disconnected);
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 0);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert!(!state.sessions.sftp_browsers[0].loading);
    assert!(
        state.sessions.sftp_browsers[0]
            .last_error
            .as_deref()
            .unwrap_or("")
            .contains("SFTP 会话已结束")
    );
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
fn backend_queue_pump_discards_pending_sftp_transfers_after_sftp_error() {
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
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/metrics.log".to_owned(),
    });
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::RemoteName,
        value: "metrics.log".to_owned(),
    });
    state.apply(Message::UploadSftp { host_id });
    state.apply(Message::RefreshSftp { host_id });
    let mut executor = FailingSftpExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 3);
    assert_eq!(state.backend_commands.pending_count(), 1);
    assert!(matches!(
        state.backend_commands.front(),
        Some(crate::backend::BackendCommand::Sftp {
            request: crate::backend::SftpRequest::ListDir { .. },
            ..
        })
    ));
    assert!(state.sessions.transfers.iter().all(|task| matches!(
        &task.status,
        TransferStatus::Failed { reason } if reason.contains("permission denied")
    )));
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Connected
    ));
}

#[test]
fn backend_queue_pump_discards_pending_sftp_writes_after_sftp_error() {
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
    state.apply(Message::RemoveSftpFile {
        host_id,
        remote_path: "/home/ops/old.log".to_owned(),
    });
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::NewDirName,
        value: "releases".to_owned(),
    });
    state.apply(Message::CreateSftpDir { host_id });
    state.apply(Message::RefreshSftp { host_id });
    let mut executor = FailingSftpExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 1);
    assert_eq!(state.backend_commands.pending_count(), 1);
    assert!(matches!(
        state.backend_commands.front(),
        Some(crate::backend::BackendCommand::Sftp {
            request: crate::backend::SftpRequest::ListDir { .. },
            ..
        })
    ));
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Connected
    ));
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
fn backend_queue_pump_skips_terminal_tunnel_start_commands() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let rule = sample_tunnel_rule();
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.storage.upsert_host(host);
    state.sessions.open_tunnel_tab(session_id, host_id, &rule);
    state
        .sessions
        .set_status(session_id, SessionStatus::Connected);
    state
        .sessions
        .start_tunnel(session_id, &rule, Some(host_id), 10);
    state
        .sessions
        .fail_tunnel_for_session_rule(session_id, &rule.name, "connection lost");
    state
        .backend_commands
        .push(crate::backend::BackendCommand::StartTunnel {
            session_id,
            request: crate::backend::TunnelStartRequest::new(rule.clone())
                .expect("测试隧道规则应有效"),
        });
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(!outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 0);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        state.sessions.tunnels[0].status,
        TunnelStatus::Failed
    ));
    assert_eq!(
        state.sessions.tunnels[0].last_error.as_deref(),
        Some("connection lost")
    );
}

#[test]
fn backend_queue_pump_skips_terminal_tunnel_stop_commands() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let rule = sample_tunnel_rule();
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.storage.upsert_host(host);
    state.sessions.open_tunnel_tab(session_id, host_id, &rule);
    state
        .sessions
        .set_status(session_id, SessionStatus::Connected);
    state
        .sessions
        .start_tunnel(session_id, &rule, Some(host_id), 10);
    state.sessions.mark_tunnel_running(session_id, &rule.name);
    state.sessions.mark_tunnel_stopping(session_id, &rule.name);
    state.sessions.stop_tunnel(session_id, &rule.name);
    state
        .backend_commands
        .push(crate::backend::BackendCommand::StopTunnel {
            session_id,
            request: crate::backend::TunnelStopRequest::by_name(rule.name.clone()),
        });
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(!outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 0);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        state.sessions.tunnels[0].status,
        TunnelStatus::Stopped
    ));
}

#[test]
fn backend_queue_pump_skips_stale_tunnel_stop_commands_for_same_rule() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let rule = sample_tunnel_rule();
    let stale_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let current_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.storage.upsert_host(host);
    state
        .sessions
        .open_tunnel_tab(current_session_id, host_id, &rule);
    state
        .sessions
        .set_status(current_session_id, SessionStatus::Connected);
    state
        .sessions
        .start_tunnel(current_session_id, &rule, Some(host_id), 20);
    state
        .sessions
        .mark_tunnel_running(current_session_id, &rule.name);
    state
        .backend_commands
        .push(crate::backend::BackendCommand::StopTunnel {
            session_id: stale_session_id,
            request: crate::backend::TunnelStopRequest::by_name(rule.name.clone()),
        });
    let mut executor = NoopBackendExecutor;

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(!outcome.changed());
    assert_eq!(outcome.executed_backend_commands, 0);
    assert_eq!(outcome.applied_backend_events, 0);
    assert!(outcome.error.is_none());
    assert!(state.backend_commands.is_empty());
    assert_eq!(state.sessions.tunnels[0].session_id, current_session_id);
    assert!(matches!(
        state.sessions.tunnels[0].status,
        TunnelStatus::Running
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
