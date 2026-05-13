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
fn output_event_appends_to_terminal_buffer() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();

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
fn transfer_progress_event_updates_existing_transfer() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let transfer_id = TransferId(Uuid::new_v4());
    let session_id = session_id();

    sessions.enqueue_transfer(TransferTask {
        id: transfer_id,
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
            transferred_bytes: 80,
            status: TransferStatus::Running,
        },
    );

    assert!(outcome.session_updated);
    assert_eq!(sessions.transfers[0].transferred_bytes, 80);
    assert!(matches!(
        sessions.transfers[0].status,
        TransferStatus::Running
    ));
}

#[test]
fn tunnel_status_event_updates_runtime_state() {
    let mut sessions = SessionManager::default();
    let mut terminal = TerminalManager::default();
    let session_id = session_id();
    let rule = tunnel_rule("local-db");

    sessions.start_tunnel(&rule, Some(host_id()), 10);
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
