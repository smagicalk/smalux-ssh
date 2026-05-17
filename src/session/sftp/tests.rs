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

    assert!(sessions.reassign_sftp_browser_session(current_host_id, second_session_id));
    assert_eq!(sessions.sftp_browsers[0].session_id, second_session_id);
    assert!(!sessions.reassign_sftp_browser_session(current_host_id, second_session_id));
    assert!(!sessions.reassign_sftp_browser_session(missing_host_id, first_session_id));
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
