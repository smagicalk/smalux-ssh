use super::*;
use crate::model::SftpEntryKind;
use uuid::Uuid;

fn host_id() -> HostId {
    HostId(Uuid::new_v4())
}

fn session_id() -> SessionId {
    SessionId(Uuid::new_v4())
}

#[test]
fn opening_sftp_tab_creates_browser_state() {
    let mut sessions = SessionManager::default();
    let id = session_id();
    let host_id = host_id();

    sessions.open_sftp_tab(id, host_id, "/home/ops");

    assert_eq!(sessions.tab_count(), 1);
    assert_eq!(sessions.sftp_browser_count(), 1);
    assert_eq!(sessions.sftp_browsers[0].session_id, id);
    assert_eq!(sessions.sftp_browsers[0].host_id, host_id);
    assert_eq!(sessions.sftp_browsers[0].current_dir, "/home/ops");
    assert!(matches!(sessions.tabs[0].kind, SessionKind::Sftp));
}

#[test]
fn sftp_entries_update_existing_browser() {
    let mut sessions = SessionManager::default();
    let host_id = host_id();

    sessions.open_sftp_tab(session_id(), host_id, "/home/ops");
    sessions.set_sftp_loading(host_id, true);

    assert!(sessions.set_sftp_entries(
        host_id,
        "/var/log",
        vec![SftpEntry {
            name: "syslog".to_owned(),
            remote_path: "/var/log/syslog".to_owned(),
            kind: SftpEntryKind::File,
            size: Some(100),
            modified_at_unix_secs: None,
            permissions: None,
        }],
    ));
    assert_eq!(sessions.sftp_browsers[0].current_dir, "/var/log");
    assert_eq!(sessions.sftp_browsers[0].entries.len(), 1);
    assert!(!sessions.sftp_browsers[0].loading);
    assert!(sessions.sftp_browsers[0].last_error.is_none());
}

#[test]
fn sftp_entries_clear_stale_selection_after_refresh() {
    let mut sessions = SessionManager::default();
    let host_id = host_id();

    sessions.open_sftp_tab(session_id(), host_id, "/home/ops");
    sessions.select_sftp_entry(host_id, "/home/ops/deploy.sh");

    assert!(sessions.set_sftp_entries(
        host_id,
        "/home/ops",
        vec![SftpEntry {
            name: "config.toml".to_owned(),
            remote_path: "/home/ops/config.toml".to_owned(),
            kind: SftpEntryKind::File,
            size: Some(128),
            modified_at_unix_secs: None,
            permissions: None,
        }],
    ));

    assert!(sessions.sftp_browsers[0].selected_path.is_none());
}

#[test]
fn sftp_entries_keep_visible_selection_after_refresh() {
    let mut sessions = SessionManager::default();
    let host_id = host_id();

    sessions.open_sftp_tab(session_id(), host_id, "/home/ops");
    sessions.select_sftp_entry(host_id, "/home/ops/deploy.sh");

    assert!(sessions.set_sftp_entries(
        host_id,
        "/home/ops",
        vec![SftpEntry {
            name: "deploy.sh".to_owned(),
            remote_path: "/home/ops/deploy.sh".to_owned(),
            kind: SftpEntryKind::File,
            size: Some(256),
            modified_at_unix_secs: None,
            permissions: None,
        }],
    ));

    assert_eq!(
        sessions.sftp_browsers[0].selected_path.as_deref(),
        Some("/home/ops/deploy.sh")
    );
}

#[test]
fn sftp_loading_state_can_be_toggled() {
    let mut sessions = SessionManager::default();
    let current_host_id = host_id();

    sessions.open_sftp_tab(session_id(), current_host_id, "/home/ops");
    sessions.fail_sftp_browser(current_host_id, "previous error");

    assert!(sessions.set_sftp_loading(current_host_id, true));
    assert!(sessions.sftp_browsers[0].loading);
    assert!(sessions.sftp_browsers[0].last_error.is_none());

    assert!(sessions.set_sftp_loading(current_host_id, false));
    assert!(!sessions.sftp_browsers[0].loading);
    assert!(!sessions.set_sftp_loading(host_id(), true));
}

#[test]
fn sftp_loading_by_session_requires_current_browser_owner() {
    let mut sessions = SessionManager::default();
    let host_id = host_id();
    let stale_session_id = session_id();
    let current_session_id = session_id();

    sessions.open_sftp_tab(stale_session_id, host_id, "/old");
    sessions.open_sftp_tab(current_session_id, host_id, "/new");
    sessions.set_sftp_loading(host_id, true);

    assert!(!sessions.set_sftp_loading_for_session(stale_session_id, false));
    assert!(sessions.sftp_browsers[0].loading);

    assert!(sessions.set_sftp_loading_for_session(current_session_id, false));
    assert!(!sessions.sftp_browsers[0].loading);
}

#[test]
fn sftp_loading_by_session_rejects_terminal_owner_when_enabling() {
    let mut sessions = SessionManager::default();
    let host_id = host_id();
    let session_id = session_id();

    sessions.open_sftp_tab(session_id, host_id, "/home/ops");
    sessions.set_status(session_id, SessionStatus::Disconnected);

    assert!(!sessions.set_sftp_loading_for_session(session_id, true));
    assert!(!sessions.sftp_browsers[0].loading);
}

#[test]
fn sftp_loading_by_session_allows_terminal_owner_cleanup() {
    let mut sessions = SessionManager::default();
    let host_id = host_id();
    let session_id = session_id();

    sessions.open_sftp_tab(session_id, host_id, "/home/ops");
    sessions.set_sftp_loading(host_id, true);
    sessions.set_status(session_id, SessionStatus::Disconnected);

    assert!(sessions.set_sftp_loading_for_session(session_id, false));
    assert!(!sessions.sftp_browsers[0].loading);
}

#[test]
fn sftp_browser_command_acceptance_requires_current_non_terminal_owner() {
    let mut sessions = SessionManager::default();
    let host_id = host_id();
    let stale_session_id = session_id();
    let current_session_id = session_id();

    sessions.open_sftp_tab(stale_session_id, host_id, "/old");
    sessions.open_sftp_tab(current_session_id, host_id, "/new");
    sessions.set_status(current_session_id, SessionStatus::Connected);

    assert!(!sessions.can_execute_sftp_browser_command(stale_session_id));
    assert!(sessions.can_execute_sftp_browser_command(current_session_id));

    sessions.set_status(current_session_id, SessionStatus::Disconnected);

    assert!(!sessions.can_execute_sftp_browser_command(current_session_id));
}

#[test]
fn sftp_transfer_command_acceptance_requires_non_terminal_sftp_tab() {
    let mut sessions = SessionManager::default();
    let host_id = host_id();
    let stale_session_id = session_id();
    let current_session_id = session_id();
    let shell_session_id = session_id();

    sessions.open_sftp_tab(stale_session_id, host_id, "/old");
    sessions.open_sftp_tab(current_session_id, host_id, "/new");
    sessions.open_shell_tab(shell_session_id, host_id, "shell");
    sessions.set_status(stale_session_id, SessionStatus::Connected);
    sessions.set_status(current_session_id, SessionStatus::Connected);

    assert!(sessions.can_execute_sftp_transfer_command(stale_session_id));
    assert!(sessions.can_execute_sftp_transfer_command(current_session_id));
    assert!(!sessions.can_execute_sftp_transfer_command(shell_session_id));

    sessions.set_status(current_session_id, SessionStatus::Disconnected);

    assert!(!sessions.can_execute_sftp_transfer_command(current_session_id));
}

#[test]
fn sftp_selection_can_be_set_and_cleared() {
    let mut sessions = SessionManager::default();
    let current_host_id = host_id();

    sessions.open_sftp_tab(session_id(), current_host_id, "/home/ops");

    assert!(sessions.select_sftp_entry(current_host_id, "/home/ops/deploy.sh"));
    assert_eq!(
        sessions.sftp_browsers[0].selected_path.as_deref(),
        Some("/home/ops/deploy.sh")
    );
    assert!(sessions.clear_sftp_selection(current_host_id));
    assert!(sessions.sftp_browsers[0].selected_path.is_none());
    assert!(!sessions.clear_sftp_selection(host_id()));
}

#[test]
fn sftp_browser_owner_can_be_reassigned() {
    let mut sessions = SessionManager::default();
    let current_host_id = host_id();
    let missing_host_id = host_id();
    let first_session_id = session_id();
    let second_session_id = session_id();

    sessions.open_sftp_tab(first_session_id, current_host_id, "/home/ops");
    sessions.open_sftp_tab(second_session_id, current_host_id, "/var/log");
    sessions.reassign_sftp_browser_session(current_host_id, first_session_id);

    assert!(sessions.reassign_sftp_browser_session(current_host_id, second_session_id));
    assert_eq!(sessions.sftp_browsers[0].session_id, second_session_id);
    assert!(!sessions.reassign_sftp_browser_session(current_host_id, second_session_id));
    assert!(!sessions.reassign_sftp_browser_session(missing_host_id, first_session_id));
}

#[test]
fn sftp_browser_owner_reassignment_requires_matching_sftp_tab() {
    let mut sessions = SessionManager::default();
    let current_host_id = host_id();
    let other_host_id = host_id();
    let first_session_id = session_id();
    let shell_session_id = session_id();
    let other_host_session_id = session_id();
    let missing_session_id = session_id();
    let disconnected_session_id = session_id();
    let failed_session_id = session_id();

    sessions.open_sftp_tab(first_session_id, current_host_id, "/home/ops");
    sessions.open_shell_tab(shell_session_id, current_host_id, "shell");
    sessions.open_sftp_tab(other_host_session_id, other_host_id, "/tmp");
    sessions.open_sftp_tab(disconnected_session_id, current_host_id, "/old");
    sessions.set_status(disconnected_session_id, SessionStatus::Disconnected);
    sessions.open_sftp_tab(failed_session_id, current_host_id, "/failed");
    sessions.set_status(
        failed_session_id,
        SessionStatus::Failed {
            reason: "network".to_owned(),
        },
    );
    sessions.reassign_sftp_browser_session(current_host_id, first_session_id);

    assert!(!sessions.reassign_sftp_browser_session(current_host_id, shell_session_id));
    assert!(!sessions.reassign_sftp_browser_session(current_host_id, other_host_session_id));
    assert!(!sessions.reassign_sftp_browser_session(current_host_id, missing_session_id));
    assert!(!sessions.reassign_sftp_browser_session(current_host_id, disconnected_session_id));
    assert!(!sessions.reassign_sftp_browser_session(current_host_id, failed_session_id));
    assert_eq!(sessions.sftp_browsers[0].session_id, first_session_id);
}

#[test]
fn sftp_browser_owner_reassigns_after_session_loss() {
    let mut sessions = SessionManager::default();
    let host_id = host_id();
    let fallback_session_id = session_id();
    let failed_session_id = session_id();

    sessions.open_sftp_tab(fallback_session_id, host_id, "/home/ops");
    sessions.set_status(fallback_session_id, SessionStatus::Connected);
    sessions.open_sftp_tab(failed_session_id, host_id, "/var/log");
    sessions.set_sftp_loading(host_id, true);
    sessions.fail_sftp_browser(host_id, "previous error");
    sessions.set_status(
        failed_session_id,
        SessionStatus::Failed {
            reason: "network".to_owned(),
        },
    );

    assert!(sessions.reassign_sftp_browser_after_session_loss(failed_session_id));
    assert_eq!(sessions.sftp_browsers[0].session_id, fallback_session_id);
    assert!(!sessions.sftp_browsers[0].loading);
    assert!(sessions.sftp_browsers[0].last_error.is_none());
    assert!(!sessions.reassign_sftp_browser_after_session_loss(failed_session_id));
}

#[test]
fn sftp_entries_can_update_by_session_id() {
    let mut sessions = SessionManager::default();
    let host_id = host_id();
    let session_id = session_id();

    sessions.open_sftp_tab(session_id, host_id, "/home/ops");

    assert!(sessions.set_sftp_entries_for_session(session_id, "/tmp", Vec::new()));
    assert_eq!(sessions.sftp_browsers[0].current_dir, "/tmp");
    assert!(!sessions.set_sftp_entries_for_session(
        SessionId(Uuid::new_v4()),
        "/missing",
        Vec::new()
    ));
}

#[test]
fn sftp_entries_by_session_ignore_stale_tab_for_same_host() {
    let mut sessions = SessionManager::default();
    let host_id = host_id();
    let stale_session_id = session_id();
    let current_session_id = session_id();

    sessions.open_sftp_tab(stale_session_id, host_id, "/old");
    sessions.open_sftp_tab(current_session_id, host_id, "/new");

    assert!(!sessions.set_sftp_entries_for_session(
        stale_session_id,
        "/old-result",
        vec![SftpEntry {
            name: "stale.log".to_owned(),
            remote_path: "/old-result/stale.log".to_owned(),
            kind: SftpEntryKind::File,
            size: Some(1),
            modified_at_unix_secs: None,
            permissions: None,
        }],
    ));
    assert_eq!(sessions.sftp_browsers[0].current_dir, "/new");
    assert!(sessions.sftp_browsers[0].entries.is_empty());

    assert!(sessions.set_sftp_entries_for_session(current_session_id, "/new-result", Vec::new()));
    assert_eq!(sessions.sftp_browsers[0].current_dir, "/new-result");
}

#[test]
fn sftp_entries_by_session_ignore_terminal_owner() {
    let mut sessions = SessionManager::default();
    let host_id = host_id();
    let owner_session_id = session_id();

    sessions.open_sftp_tab(owner_session_id, host_id, "/home/ops");
    sessions.set_status(owner_session_id, SessionStatus::Disconnected);

    assert!(!sessions.set_sftp_entries_for_session(owner_session_id, "/late-result", Vec::new()));
    assert_eq!(sessions.sftp_browsers[0].current_dir, "/home/ops");
}

#[test]
fn sftp_entries_by_session_ignore_non_sftp_tabs() {
    let mut sessions = SessionManager::default();
    let host_id = host_id();
    let owner_session_id = session_id();

    sessions.open_sftp_tab(owner_session_id, host_id, "/home/ops");
    sessions.open_shell_tab(owner_session_id, host_id, "shell replacement");

    assert!(!sessions.set_sftp_entries_for_session(owner_session_id, "/shell-result", Vec::new()));
    assert_eq!(sessions.sftp_browsers[0].current_dir, "/home/ops");
}

#[test]
fn sftp_browser_records_failure() {
    let mut sessions = SessionManager::default();
    let current_host_id = host_id();

    sessions.open_sftp_tab(session_id(), current_host_id, "/home/ops");

    assert!(sessions.fail_sftp_browser(current_host_id, "permission denied"));
    assert_eq!(
        sessions.sftp_browsers[0].last_error.as_deref(),
        Some("permission denied")
    );
    assert!(!sessions.fail_sftp_browser(host_id(), "missing"));
}

#[test]
fn sftp_failure_can_update_by_session_id() {
    let mut sessions = SessionManager::default();
    let host_id = host_id();
    let session_id = session_id();

    sessions.open_sftp_tab(session_id, host_id, "/home/ops");

    assert!(sessions.fail_sftp_browser_for_session(session_id, "network"));
    assert_eq!(
        sessions.sftp_browsers[0].last_error.as_deref(),
        Some("network")
    );
}

#[test]
fn sftp_failure_by_session_ignores_stale_tab_for_same_host() {
    let mut sessions = SessionManager::default();
    let host_id = host_id();
    let stale_session_id = session_id();
    let current_session_id = session_id();

    sessions.open_sftp_tab(stale_session_id, host_id, "/old");
    sessions.open_sftp_tab(current_session_id, host_id, "/new");

    assert!(!sessions.fail_sftp_browser_for_session(stale_session_id, "late failure"));
    assert!(sessions.sftp_browsers[0].last_error.is_none());

    assert!(sessions.fail_sftp_browser_for_session(current_session_id, "current failure"));
    assert_eq!(
        sessions.sftp_browsers[0].last_error.as_deref(),
        Some("current failure")
    );
}

#[test]
fn sftp_failure_by_session_ignores_non_sftp_tabs() {
    let mut sessions = SessionManager::default();
    let host_id = host_id();
    let sftp_session_id = session_id();
    let shell_session_id = session_id();

    sessions.open_sftp_tab(sftp_session_id, host_id, "/home/ops");
    sessions.open_shell_tab(shell_session_id, host_id, "shell");

    assert!(!sessions.fail_sftp_browser_for_session(shell_session_id, "shell failed"));
    assert!(sessions.sftp_browsers[0].last_error.is_none());
}

#[test]
fn sftp_operation_failure_ignores_terminal_owner() {
    let mut sessions = SessionManager::default();
    let host_id = host_id();
    let session_id = session_id();

    sessions.open_sftp_tab(session_id, host_id, "/home/ops");
    sessions.set_sftp_loading(host_id, true);
    sessions.fail_sftp_browser_for_session(session_id, "SFTP 会话已断开");
    sessions.set_status(session_id, SessionStatus::Disconnected);

    assert!(!sessions.fail_sftp_operation_for_session(session_id, "late permission denied"));
    assert_eq!(
        sessions.sftp_browsers[0].last_error.as_deref(),
        Some("SFTP 会话已断开")
    );
}

#[test]
fn sftp_operation_failure_records_current_owner_error() {
    let mut sessions = SessionManager::default();
    let host_id = host_id();
    let session_id = session_id();

    sessions.open_sftp_tab(session_id, host_id, "/home/ops");
    sessions.set_status(session_id, SessionStatus::Connected);

    assert!(sessions.fail_sftp_operation_for_session(session_id, "permission denied"));
    assert_eq!(
        sessions.sftp_browsers[0].last_error.as_deref(),
        Some("permission denied")
    );
}
