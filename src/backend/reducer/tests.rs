use super::*;
use crate::model::{
    HostId, SessionId, SftpEntry, SftpEntryKind, TransferDirection, TransferId, TransferStatus,
    TransferTask, TunnelKind, TunnelRule,
};
use crate::terminal::TerminalTabState;
use uuid::Uuid;

fn host_id() -> HostId {
    HostId(Uuid::new_v4())
}

fn session_id() -> SessionId {
    SessionId(Uuid::new_v4())
}

fn tunnel_rule(name: &str) -> TunnelRule {
    TunnelRule {
        name: name.to_owned(),
        kind: TunnelKind::Local,
        bind_host: "127.0.0.1".to_owned(),
        bind_port: 15432,
        target_host: "10.0.0.5".to_owned(),
        target_port: 5432,
        auto_start: false,
    }
}

#[test]
fn connected_event_updates_session_status() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();

    sessions.open_shell_tab(session_id, host_id(), "production");
    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::Connected { session_id },
    );

    assert!(outcome.changed());
    assert!(matches!(sessions.tabs[0].status, SessionStatus::Connected));
}

#[test]
fn connected_event_ignores_terminal_session() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();

    sessions.open_shell_tab(session_id, host_id(), "production");
    sessions.set_status(
        session_id,
        SessionStatus::Failed {
            reason: "network".to_owned(),
        },
    );
    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::Connected { session_id },
    );

    assert!(!outcome.changed());
    assert!(matches!(
        &sessions.tabs[0].status,
        SessionStatus::Failed { reason } if reason == "network"
    ));
    assert!(sessions.active.is_empty());
}

#[test]
fn remote_connection_events_ignore_local_shell_session() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();

    sessions.open_local_shell_tab(session_id, crate::model::DEFAULT_LOCAL_TERMINAL_TITLE);
    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::Connecting {
            session_id,
            endpoint: "example.com:22".to_owned(),
        },
    );

    assert!(!outcome.changed());
    assert!(matches!(sessions.tabs[0].status, SessionStatus::Connected));

    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::Authenticating {
            session_id,
            username: "deploy".to_owned(),
        },
    );

    assert!(!outcome.changed());
    assert!(matches!(sessions.tabs[0].status, SessionStatus::Connected));
}

#[test]
fn connected_event_accepts_local_shell_session() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();

    sessions.open_local_shell_tab(session_id, crate::model::DEFAULT_LOCAL_TERMINAL_TITLE);
    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::Connected { session_id },
    );

    assert!(outcome.session_updated);
    assert!(matches!(sessions.tabs[0].status, SessionStatus::Connected));
}

#[test]
fn connection_lifecycle_events_update_session_status() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();

    sessions.open_remote_command_tab(session_id, host_id(), "uptime", None);

    apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::Authenticating {
            session_id,
            username: "deploy".to_owned(),
        },
    );
    assert!(matches!(
        sessions.tabs[0].status,
        SessionStatus::Authenticating
    ));

    apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::RemoteCommandStarted {
            session_id,
            command: "uptime".to_owned(),
        },
    );
    assert!(matches!(
        sessions.tabs[0].status,
        SessionStatus::RunningCommand
    ));
}

#[test]
fn remote_command_started_ignores_non_command_session() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();

    sessions.open_shell_tab(session_id, host_id(), "production");
    sessions.set_status(session_id, SessionStatus::Connected);

    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::RemoteCommandStarted {
            session_id,
            command: "uptime".to_owned(),
        },
    );

    assert!(!outcome.changed());
    assert!(matches!(sessions.tabs[0].status, SessionStatus::Connected));
}

#[test]
fn shell_opened_marks_shell_session_connected() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();

    sessions.open_shell_tab(session_id, host_id(), "production");
    apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::ShellOpened { session_id },
    );

    assert!(matches!(sessions.tabs[0].status, SessionStatus::Connected));
}

#[test]
fn shell_opened_ignores_non_shell_session() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();

    sessions.open_remote_command_tab(session_id, host_id(), "uptime", None);

    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::ShellOpened { session_id },
    );

    assert!(!outcome.changed());
    assert!(matches!(sessions.tabs[0].status, SessionStatus::Created));
}

#[test]
fn output_event_appends_to_terminal_buffer() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();

    sessions.open_shell_tab(session_id, host_id(), "production");
    sessions.set_status(session_id, SessionStatus::Connected);
    terminal.open_tab(TerminalTabState::new(session_id, "production"));
    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::Output {
            session_id,
            line: "hello".to_owned(),
        },
    );

    assert!(outcome.terminal_updated);
    assert_eq!(terminal.tabs[0].buffer, vec!["hello"]);
}

#[test]
fn output_event_ignores_terminal_session() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();

    sessions.open_shell_tab(session_id, host_id(), "production");
    sessions.set_status(
        session_id,
        SessionStatus::Failed {
            reason: "network".to_owned(),
        },
    );
    terminal.open_tab(TerminalTabState::new(session_id, "production"));
    terminal.append_output(session_id, "old output");
    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::Output {
            session_id,
            line: "late output".to_owned(),
        },
    );

    assert!(!outcome.changed());
    assert_eq!(terminal.tabs[0].buffer, vec!["old output"]);
}

#[test]
fn local_terminal_output_drops_duplicate_shell_echo() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = crate::model::LOCAL_TERMINAL_SESSION_ID;
    let prompt = crate::backend::LocalShellProfile::default_for_platform().prompt;

    sessions.open_local_shell_tab(session_id, crate::model::DEFAULT_LOCAL_TERMINAL_TITLE);
    terminal.open_tab(TerminalTabState::new(session_id, "local"));
    terminal.append_local_echo(session_id, prompt, "ls\n");
    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::Output {
            session_id,
            line: "ls".to_owned(),
        },
    );

    assert!(!outcome.terminal_updated);
    assert_eq!(terminal.tabs[0].buffer, vec![format!("{prompt} ls")]);
}

#[test]
fn clear_terminal_event_clears_terminal_buffer() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();

    sessions.open_shell_tab(session_id, host_id(), "production");
    sessions.set_status(session_id, SessionStatus::Connected);
    terminal.open_tab(TerminalTabState::new(session_id, "production"));
    terminal.append_output(session_id, "old output");
    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::ClearTerminal { session_id },
    );

    assert!(outcome.terminal_updated);
    assert!(terminal.tabs[0].buffer.is_empty());
}

#[test]
fn clear_terminal_event_ignores_terminal_session() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();

    sessions.open_shell_tab(session_id, host_id(), "production");
    sessions.set_status(session_id, SessionStatus::Disconnected);
    terminal.open_tab(TerminalTabState::new(session_id, "production"));
    terminal.append_output(session_id, "old output");
    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::ClearTerminal { session_id },
    );

    assert!(!outcome.changed());
    assert_eq!(terminal.tabs[0].buffer, vec!["old output"]);
}

#[test]
fn command_exited_marks_process_session_terminal() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();

    sessions.open_remote_command_tab(session_id, host_id(), "uptime", None);
    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::CommandExited {
            session_id,
            exit_code: Some(3),
        },
    );

    assert!(outcome.session_updated);
    assert!(matches!(
        &sessions.tabs[0].status,
        SessionStatus::Failed { reason } if reason == "remote command exited with 3"
    ));
}

#[test]
fn command_exited_ignores_non_process_session() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();

    sessions.open_sftp_tab(session_id, host_id(), "/home/ops");
    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::CommandExited {
            session_id,
            exit_code: Some(0),
        },
    );

    assert!(!outcome.changed());
    assert!(matches!(sessions.tabs[0].status, SessionStatus::Created));
}

#[test]
fn sftp_entries_event_updates_browser_by_session() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();

    sessions.open_sftp_tab(session_id, host_id(), "/home/ops");
    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::SftpEntries {
            session_id,
            remote_path: "/var/log".to_owned(),
            entries: vec![SftpEntry {
                name: "syslog".to_owned(),
                remote_path: "/var/log/syslog".to_owned(),
                kind: SftpEntryKind::File,
                size: Some(100),
                modified_at_unix_secs: None,
                permissions: None,
            }],
        },
    );

    assert!(outcome.session_updated);
    assert_eq!(sessions.sftp_browsers[0].current_dir, "/var/log");
    assert_eq!(sessions.sftp_browsers[0].entries.len(), 1);
}

#[test]
fn sftp_entries_event_ignores_terminal_sftp_session() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();

    sessions.open_sftp_tab(session_id, host_id(), "/home/ops");
    sessions.set_status(session_id, SessionStatus::Disconnected);
    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::SftpEntries {
            session_id,
            remote_path: "/late-result".to_owned(),
            entries: Vec::new(),
        },
    );

    assert!(!outcome.changed());
    assert_eq!(sessions.sftp_browsers[0].current_dir, "/home/ops");
}

#[test]
fn transfer_progress_event_updates_existing_transfer() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let transfer_id = TransferId(Uuid::new_v4());
    let session_id = session_id();

    sessions.enqueue_transfer(TransferTask {
        id: transfer_id,
        session_id,
        host_id: host_id(),
        direction: TransferDirection::Download,
        local_path: "C:/tmp/syslog".to_owned(),
        remote_path: "/var/log/syslog".to_owned(),
        total_bytes: Some(100),
        transferred_bytes: 0,
        status: TransferStatus::Queued,
    });
    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::TransferProgress {
            session_id,
            transfer_id,
            total_bytes: Some(120),
            transferred_bytes: 80,
            status: TransferStatus::Running,
        },
    );

    assert!(outcome.session_updated);
    assert_eq!(sessions.transfers[0].total_bytes, Some(120));
    assert_eq!(sessions.transfers[0].transferred_bytes, 80);
    assert!(matches!(
        sessions.transfers[0].status,
        TransferStatus::Running
    ));
}

#[test]
fn transfer_progress_event_ignores_cancelled_transfer() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let transfer_id = TransferId(Uuid::new_v4());
    let session_id = session_id();

    sessions.enqueue_transfer(TransferTask {
        id: transfer_id,
        session_id,
        host_id: host_id(),
        direction: TransferDirection::Download,
        local_path: "C:/tmp/syslog".to_owned(),
        remote_path: "/var/log/syslog".to_owned(),
        total_bytes: Some(100),
        transferred_bytes: 0,
        status: TransferStatus::Cancelled,
    });
    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::TransferProgress {
            session_id,
            transfer_id,
            total_bytes: Some(120),
            transferred_bytes: 80,
            status: TransferStatus::Running,
        },
    );

    assert!(!outcome.changed());
    assert_eq!(sessions.transfers[0].transferred_bytes, 0);
    assert!(matches!(
        sessions.transfers[0].status,
        TransferStatus::Cancelled
    ));
}

#[test]
fn transfer_progress_event_requires_matching_session_owner() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let transfer_id = TransferId(Uuid::new_v4());
    let owner_session_id = session_id();
    let stale_session_id = session_id();

    sessions.enqueue_transfer(TransferTask {
        id: transfer_id,
        session_id: owner_session_id,
        host_id: host_id(),
        direction: TransferDirection::Download,
        local_path: "C:/tmp/syslog".to_owned(),
        remote_path: "/var/log/syslog".to_owned(),
        total_bytes: Some(100),
        transferred_bytes: 0,
        status: TransferStatus::Queued,
    });
    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::TransferProgress {
            session_id: stale_session_id,
            transfer_id,
            total_bytes: Some(120),
            transferred_bytes: 80,
            status: TransferStatus::Running,
        },
    );

    assert!(!outcome.changed());
    assert_eq!(sessions.transfers[0].total_bytes, Some(100));
    assert_eq!(sessions.transfers[0].transferred_bytes, 0);
    assert!(matches!(
        sessions.transfers[0].status,
        TransferStatus::Queued
    ));
}

#[test]
fn sftp_failed_event_records_browser_error_without_failing_transfers() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let current_session_id = session_id();
    let current_host_id = host_id();

    sessions.open_sftp_tab(current_session_id, current_host_id, "/home/ops");
    sessions.set_sftp_loading(current_host_id, true);
    sessions.enqueue_transfer(TransferTask {
        id: TransferId(Uuid::new_v4()),
        session_id: current_session_id,
        host_id: current_host_id,
        direction: TransferDirection::Download,
        local_path: "C:/tmp/syslog".to_owned(),
        remote_path: "/var/log/syslog".to_owned(),
        total_bytes: Some(100),
        transferred_bytes: 0,
        status: TransferStatus::Queued,
    });

    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::SftpFailed {
            session_id: current_session_id,
            reason: "permission denied".to_owned(),
        },
    );

    assert!(outcome.session_updated);
    assert!(!sessions.sftp_browsers[0].loading);
    assert_eq!(
        sessions.sftp_browsers[0].last_error.as_deref(),
        Some("permission denied")
    );
    assert!(matches!(
        sessions.transfers[0].status,
        TransferStatus::Queued
    ));
}

#[test]
fn sftp_failed_event_ignores_terminal_sftp_session() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();

    sessions.open_sftp_tab(session_id, host_id(), "/home/ops");
    sessions.set_sftp_loading(sessions.tabs[0].host_id.unwrap(), true);
    apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::Disconnected { session_id },
    );

    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::SftpFailed {
            session_id,
            reason: "late permission denied".to_owned(),
        },
    );

    assert!(!outcome.changed());
    assert_eq!(
        sessions.sftp_browsers[0].last_error.as_deref(),
        Some("SFTP 会话已断开")
    );
}

#[test]
fn tunnel_status_event_updates_runtime_state() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();
    let host_id = host_id();
    let rule = tunnel_rule("local-db");

    sessions.open_tunnel_tab(session_id, host_id, &rule);
    sessions.start_tunnel(session_id, &rule, Some(host_id), 10);
    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::TunnelStatusChanged {
            session_id,
            rule_name: "local-db".to_owned(),
            status: TunnelStatus::Running,
        },
    );

    assert!(outcome.session_updated);
    assert!(matches!(sessions.tunnels[0].status, TunnelStatus::Running));
    assert!(matches!(sessions.tabs[0].status, SessionStatus::Connected));
}

#[test]
fn tunnel_status_event_ignores_stale_session_for_same_rule() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let stale_session_id = session_id();
    let current_session_id = session_id();
    let host_id = host_id();
    let rule = tunnel_rule("local-db");

    sessions.open_tunnel_tab(current_session_id, host_id, &rule);
    sessions.start_tunnel(current_session_id, &rule, Some(host_id), 10);

    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::TunnelStatusChanged {
            session_id: stale_session_id,
            rule_name: "local-db".to_owned(),
            status: TunnelStatus::Stopped,
        },
    );

    assert!(!outcome.changed());
    assert!(matches!(sessions.tunnels[0].status, TunnelStatus::Starting));
}

#[test]
fn tunnel_status_event_ignores_rule_mismatch_for_session() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();
    let host_id = host_id();
    let rule = tunnel_rule("local-db");

    sessions.open_tunnel_tab(session_id, host_id, &rule);
    sessions.start_tunnel(session_id, &rule, Some(host_id), 10);

    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::TunnelStatusChanged {
            session_id,
            rule_name: "metrics".to_owned(),
            status: TunnelStatus::Running,
        },
    );

    assert!(!outcome.changed());
    assert!(matches!(sessions.tunnels[0].status, TunnelStatus::Starting));
    assert!(!matches!(sessions.tabs[0].status, SessionStatus::Connected));
}

#[test]
fn tunnel_stopped_event_marks_session_disconnected() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();
    let host_id = host_id();
    let rule = tunnel_rule("local-db");

    sessions.open_tunnel_tab(session_id, host_id, &rule);
    sessions.start_tunnel(session_id, &rule, Some(host_id), 10);
    sessions.set_status(session_id, SessionStatus::Connected);

    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::TunnelStatusChanged {
            session_id,
            rule_name: "local-db".to_owned(),
            status: TunnelStatus::Stopped,
        },
    );

    assert!(outcome.session_updated);
    assert!(matches!(sessions.tunnels[0].status, TunnelStatus::Stopped));
    assert!(matches!(
        sessions.tabs[0].status,
        SessionStatus::Disconnected
    ));
}

#[test]
fn failed_event_marks_session_failed() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();

    sessions.open_shell_tab(session_id, host_id(), "production");
    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::Failed {
            session_id,
            reason: "network".to_owned(),
        },
    );

    assert!(outcome.session_updated);
    assert!(matches!(
        &sessions.tabs[0].status,
        SessionStatus::Failed { reason } if reason == "network"
    ));
}

#[test]
fn failed_sftp_event_records_browser_error() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();

    sessions.open_sftp_tab(session_id, host_id(), "/home/ops");
    sessions.set_sftp_loading(sessions.tabs[0].host_id.unwrap(), true);

    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::Failed {
            session_id,
            reason: "permission denied".to_owned(),
        },
    );

    assert!(outcome.session_updated);
    assert!(!sessions.sftp_browsers[0].loading);
    assert_eq!(
        sessions.sftp_browsers[0].last_error.as_deref(),
        Some("permission denied")
    );
}

#[test]
fn disconnected_sftp_event_stops_browser_loading() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();

    sessions.open_sftp_tab(session_id, host_id(), "/home/ops");
    sessions.set_sftp_loading(sessions.tabs[0].host_id.unwrap(), true);

    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::Disconnected { session_id },
    );

    assert!(outcome.session_updated);
    assert!(!sessions.sftp_browsers[0].loading);
    assert_eq!(
        sessions.sftp_browsers[0].last_error.as_deref(),
        Some("SFTP 会话已断开")
    );
}

#[test]
fn disconnected_sftp_event_reassigns_browser_to_available_session() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let host_id = host_id();
    let fallback_session_id = session_id();
    let disconnected_session_id = session_id();

    sessions.open_sftp_tab(fallback_session_id, host_id, "/home/ops");
    sessions.set_status(fallback_session_id, SessionStatus::Connected);
    sessions.open_sftp_tab(disconnected_session_id, host_id, "/var/log");
    sessions.set_status(disconnected_session_id, SessionStatus::Connected);

    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::Disconnected {
            session_id: disconnected_session_id,
        },
    );

    assert!(outcome.session_updated);
    assert_eq!(sessions.sftp_browsers[0].session_id, fallback_session_id);
    assert!(!sessions.sftp_browsers[0].loading);
    assert!(sessions.sftp_browsers[0].last_error.is_none());
}

#[test]
fn disconnected_tunnel_event_stops_runtime_and_marks_session_disconnected() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();
    let host_id = host_id();
    let rule = tunnel_rule("local-db");

    sessions.open_tunnel_tab(session_id, host_id, &rule);
    sessions.start_tunnel(session_id, &rule, Some(host_id), 10);
    sessions.set_status(session_id, SessionStatus::Connected);

    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::Disconnected { session_id },
    );

    assert!(outcome.session_updated);
    assert!(matches!(sessions.tunnels[0].status, TunnelStatus::Stopped));
    assert!(matches!(
        sessions.tabs[0].status,
        SessionStatus::Disconnected
    ));
}

#[test]
fn disconnected_sftp_event_fails_owned_transfers_only() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let current_session_id = session_id();
    let other_session_id = session_id();
    let current_host_id = host_id();
    let other_host_id = host_id();

    sessions.open_sftp_tab(current_session_id, current_host_id, "/home/ops");
    sessions.open_sftp_tab(other_session_id, other_host_id, "/var/log");
    sessions.enqueue_transfer(TransferTask {
        id: TransferId(Uuid::new_v4()),
        session_id: current_session_id,
        host_id: current_host_id,
        direction: TransferDirection::Download,
        local_path: "C:/tmp/syslog".to_owned(),
        remote_path: "/var/log/syslog".to_owned(),
        total_bytes: Some(100),
        transferred_bytes: 0,
        status: TransferStatus::Queued,
    });
    sessions.enqueue_transfer(TransferTask {
        id: TransferId(Uuid::new_v4()),
        session_id: other_session_id,
        host_id: other_host_id,
        direction: TransferDirection::Download,
        local_path: "C:/tmp/auth.log".to_owned(),
        remote_path: "/var/log/auth.log".to_owned(),
        total_bytes: Some(100),
        transferred_bytes: 0,
        status: TransferStatus::Queued,
    });

    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::Disconnected {
            session_id: current_session_id,
        },
    );

    assert!(outcome.session_updated);
    assert!(matches!(
        &sessions.transfers[0].status,
        TransferStatus::Failed { reason } if reason == "SFTP 会话已断开"
    ));
    assert!(matches!(
        sessions.transfers[1].status,
        TransferStatus::Queued
    ));
}

#[test]
fn failed_event_marks_tunnel_runtime_failed() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();
    let host_id = host_id();
    let rule = tunnel_rule("local-db");

    sessions.open_tunnel_tab(session_id, host_id, &rule);
    sessions.start_tunnel(session_id, &rule, Some(host_id), 10);

    let outcome = apply_backend_event(
        &mut sessions,
        &mut terminal,
        BackendEvent::Failed {
            session_id,
            reason: "bind failed".to_owned(),
        },
    );

    assert!(outcome.session_updated);
    assert!(matches!(sessions.tunnels[0].status, TunnelStatus::Failed));
    assert_eq!(
        sessions.tunnels[0].last_error.as_deref(),
        Some("bind failed")
    );
}
