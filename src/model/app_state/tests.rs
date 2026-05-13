use super::*;
use crate::backend::{BackendCommand, BackendEvent};
use crate::model::{AuthProfile, Host, SecretRef};

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
fn open_shell_message_creates_tabs_and_queues_backend_commands() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    let outcome = state.apply(Message::OpenShell { host_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 2);
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.terminal.tab_count(), 1);
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Connecting
    ));
    assert_eq!(state.backend_commands.pending_count(), 2);

    let commands = state.backend_commands.drain();
    let session_id = state.sessions.tabs[0].id;
    assert!(matches!(
        &commands[0],
        BackendCommand::Connect {
            session_id: command_session_id,
            target,
        } if *command_session_id == session_id
            && target.host_id == host_id
            && target.endpoint() == "example.com:22"
    ));
    assert!(matches!(
        &commands[1],
        BackendCommand::OpenShell {
            session_id: command_session_id,
            pty,
        } if *command_session_id == session_id && pty.term == "xterm-256color"
    ));
}

#[test]
fn open_shell_message_reports_missing_host_without_queueing_commands() {
    let mut state = AppState::default();
    let host_id = HostId(uuid::Uuid::new_v4());

    let outcome = state.apply(Message::OpenShell { host_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert_eq!(state.sessions.tab_count(), 0);
    assert!(state.backend_commands.is_empty());
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
fn password_host_can_still_open_shell_without_exposing_secret() {
    let mut state = AppState::default();
    let mut host = sample_host();
    host.auth = AuthProfile::Password {
        username: "root".to_owned(),
        secret: SecretRef("password:root".to_owned()),
    };
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenShell { host_id });

    let commands = state.backend_commands.drain();
    assert!(matches!(
        &commands[0],
        BackendCommand::Connect { target, .. }
            if target.auth.username() == "root"
    ));
}
