use super::*;
use crate::backend::{BackendCommand, BackendEvent};
use crate::model::{
    AuthProfile, Host, SessionStatus, TunnelKind, TunnelRule, TunnelRuntimeState, TunnelStatus,
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

#[test]
fn default_state_starts_empty_and_dark() {
    let state = AppState::default();

    assert_eq!(state.config.app_name, "smagicalssh");
    assert_eq!(state.sessions.active_count(), 0);
    assert_eq!(state.storage.host_count(), 0);
    assert_eq!(state.terminal.tab_count(), 0);
    assert_eq!(state.backend_commands.pending_count(), 0);
    assert!(matches!(state.theme, Theme::Dark));
}

#[test]
fn boot_returns_default_state_without_startup_task() {
    let (state, _task) = AppState::boot();

    assert_eq!(state.config.app_name, "smagicalssh");
    assert!(matches!(state.theme, Theme::Dark));
}

#[test]
fn toggle_theme_switches_between_dark_and_light() {
    let mut state = AppState::default();

    let outcome = state.apply(Message::ToggleTheme);
    assert!(outcome.changed());
    assert!(matches!(state.theme, Theme::Light));

    state.apply(Message::ToggleTheme);
    assert!(matches!(state.theme, Theme::Dark));
}

#[test]
fn backend_event_message_updates_existing_session_state() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenShell { host_id });
    state.backend_commands.drain();
    let session_id = state.sessions.tabs[0].id;

    let outcome = state.apply(Message::BackendEventReceived(BackendEvent::Connected {
        session_id,
    }));

    assert!(outcome.changed());
    assert_eq!(outcome.applied_backend_events, 1);
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Connected
    ));
}

#[test]
fn remove_credential_message_updates_storage() {
    let mut state = AppState::default();
    state
        .storage
        .upsert_credential(crate::model::CredentialMetadata {
            name: "deploy".to_owned(),
            kind: crate::model::CredentialKind::Password,
            username: Some("deploy".to_owned()),
            secret: Some(crate::model::SecretRef("password:deploy".to_owned())),
            key_algorithm: None,
            fingerprint: None,
        });

    let outcome = state.apply(Message::RemoveCredential {
        name: "deploy".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(state.storage.credential_count(), 0);
}

#[test]
fn trust_known_host_message_marks_entry_trusted() {
    let mut state = AppState::default();
    state
        .storage
        .upsert_known_host(crate::model::KnownHostEntry {
            host: "example.com".to_owned(),
            port: 22,
            key_algorithm: crate::model::KeyAlgorithm::Ed25519,
            fingerprint: "SHA256:demo".to_owned(),
            trusted: false,
        });

    let outcome = state.apply(Message::TrustKnownHost {
        host: "example.com".to_owned(),
        port: 22,
    });

    assert!(outcome.changed());
    assert!(state.storage.known_hosts[0].trusted);
}

#[test]
fn activate_terminal_tab_message_switches_active_tab() {
    let mut state = AppState::default();
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state
        .terminal
        .open_tab(crate::terminal::TerminalTabState::new(
            session_id,
            "production",
        ));

    let outcome = state.apply(Message::ActivateTerminalTab { session_id });

    assert!(outcome.changed());
    assert_eq!(state.terminal.active_tab, Some(session_id));
    assert_eq!(state.sessions.active_tab, Some(session_id));
}

#[test]
fn close_session_tab_message_closes_shell_and_queues_disconnect() {
    let mut state = AppState::default();
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state
        .sessions
        .open_shell_tab(session_id, host_id, "production");
    state
        .terminal
        .open_tab(crate::terminal::TerminalTabState::new(
            session_id,
            "production",
        ));

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert_eq!(state.sessions.tab_count(), 0);
    assert_eq!(state.terminal.tab_count(), 0);
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::Disconnect { session_id: queued_session_id })
            if *queued_session_id == session_id
    ));
}

#[test]
fn close_session_tab_message_removes_last_sftp_browser_for_host() {
    let mut state = AppState::default();
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state
        .sessions
        .open_sftp_tab(session_id, host_id, "/home/ops");

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert_eq!(state.sessions.tab_count(), 0);
    assert_eq!(state.sessions.sftp_browser_count(), 0);
}

#[test]
fn close_session_tab_message_keeps_sftp_browser_when_same_host_tab_remains() {
    let mut state = AppState::default();
    let first_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let second_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state.sessions.open_sftp_tab(first_id, host_id, "/home/ops");
    state.sessions.open_sftp_tab(second_id, host_id, "/var/log");

    let outcome = state.apply(Message::CloseSessionTab {
        session_id: first_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.sessions.sftp_browser_count(), 1);
    assert_eq!(state.sessions.sftp_browsers[0].current_dir, "/var/log");
}

#[test]
fn close_session_tab_message_reports_missing_tab() {
    let mut state = AppState::default();
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(!outcome.state_changed);
    assert!(outcome.error.is_some());
    assert!(state.backend_commands.is_empty());
}

#[test]
fn close_session_tab_message_requires_stopping_running_tunnel_first() {
    let mut state = AppState::default();
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    let rule = TunnelRule {
        name: "local-db".to_owned(),
        kind: TunnelKind::Local,
        bind_host: "127.0.0.1".to_owned(),
        bind_port: 15432,
        target_host: "10.0.0.5".to_owned(),
        target_port: 5432,
        auto_start: false,
    };
    state.sessions.open_tunnel_tab(session_id, host_id, &rule);
    state.sessions.tunnels.push(TunnelRuntimeState {
        rule_name: "local-db".to_owned(),
        host_id: Some(host_id),
        status: TunnelStatus::Running,
        started_at_unix_secs: Some(1),
        last_error: None,
    });

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(!outcome.state_changed);
    assert!(outcome.error.is_some());
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.sessions.tunnel_runtime_count(), 1);
    assert!(state.backend_commands.is_empty());
}

#[test]
fn send_terminal_input_message_queues_shell_input_and_records_history() {
    let mut state = AppState::default();
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state
        .sessions
        .open_shell_tab(session_id, host_id, "production");
    state
        .terminal
        .open_tab(crate::terminal::TerminalTabState::new(
            session_id,
            "production",
        ));
    state.ui.set_terminal_input(session_id, "ls");

    let outcome = state.apply(Message::SendTerminalInput { session_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert_eq!(state.storage.command_history_count(), 1);
    assert_eq!(state.ui.terminal_input_for(session_id), "");
    assert!(matches!(
        state.backend_commands.front(),
        Some(crate::backend::BackendCommand::SendShellInput { session_id: queued_session_id, input })
            if *queued_session_id == session_id && input == "ls\n"
    ));
}

#[test]
fn select_sftp_entry_message_updates_browser_selection() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.storage.upsert_host(host);
    state
        .sessions
        .open_sftp_tab(session_id, host_id, "/home/ops");

    let outcome = state.apply(Message::SelectSftpEntry {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(
        state.sessions.sftp_browsers[0].selected_path.as_deref(),
        Some("/home/ops/deploy.sh")
    );
    assert!(state.backend_commands.is_empty());
}
