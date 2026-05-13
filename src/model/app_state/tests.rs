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
