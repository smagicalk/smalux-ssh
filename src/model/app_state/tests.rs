use super::*;
use crate::backend::BackendEvent;
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
