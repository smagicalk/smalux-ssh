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
fn default_state_starts_empty() {
    let state = AppState::default();

    assert_eq!(state.config.app_name, "smagicalssh");
    assert_eq!(state.sessions.active_count(), 0);
    assert_eq!(state.storage.host_count(), 0);
    assert_eq!(state.terminal.tab_count(), 0);
    assert_eq!(state.backend_commands.pending_count(), 0);
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
fn remove_known_host_message_deletes_entry() {
    let mut state = AppState::default();
    state
        .storage
        .upsert_known_host(crate::model::KnownHostEntry::untrusted(
            "example.com",
            22,
            crate::model::KeyAlgorithm::Ed25519,
            "SHA256:demo",
        ));

    let outcome = state.apply(Message::RemoveKnownHost {
        host: "example.com".to_owned(),
        port: 22,
    });

    assert!(outcome.changed());
    assert_eq!(state.storage.known_host_count(), 0);
}

#[test]
fn activate_terminal_tab_message_switches_active_tab() {
    let mut state = AppState::default();
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state
        .sessions
        .open_shell_tab(session_id, host_id, "production");
    state
        .sessions
        .set_status(session_id, SessionStatus::Connected);
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
fn activate_sftp_tab_message_switches_session_without_terminal_tab() {
    let mut state = AppState::default();
    let first_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let second_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state.sessions.open_sftp_tab(first_session_id, host_id, "/");
    state
        .sessions
        .open_sftp_tab(second_session_id, host_id, "/var/log");

    let outcome = state.apply(Message::ActivateTerminalTab {
        session_id: first_session_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.active_tab, Some(first_session_id));
    assert!(state.terminal.active_tab.is_none());
}

#[test]
fn activate_sftp_tab_message_reassigns_browser_owner() {
    let mut state = AppState::default();
    let first_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let second_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state.sessions.open_sftp_tab(first_session_id, host_id, "/");
    state
        .sessions
        .open_sftp_tab(second_session_id, host_id, "/var/log");
    assert_eq!(
        state.sessions.sftp_browsers[0].session_id,
        second_session_id
    );

    let outcome = state.apply(Message::ActivateTerminalTab {
        session_id: first_session_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.active_tab, Some(first_session_id));
    assert_eq!(state.sessions.sftp_browsers[0].session_id, first_session_id);
}

#[test]
fn activate_disconnected_sftp_tab_keeps_available_browser_owner() {
    let mut state = AppState::default();
    let connected_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let disconnected_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state
        .sessions
        .open_sftp_tab(disconnected_session_id, host_id, "/old");
    state
        .sessions
        .set_status(disconnected_session_id, SessionStatus::Disconnected);
    state
        .sessions
        .open_sftp_tab(connected_session_id, host_id, "/current");
    state
        .sessions
        .set_status(connected_session_id, SessionStatus::Connected);

    let outcome = state.apply(Message::ActivateTerminalTab {
        session_id: disconnected_session_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.active_tab, Some(disconnected_session_id));
    assert_eq!(
        state.sessions.sftp_browsers[0].session_id,
        connected_session_id
    );
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
        .sessions
        .set_status(session_id, SessionStatus::Connected);
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
fn close_pending_shell_tab_removes_launch_commands_without_disconnect() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenShell { host_id });
    let session_id = state.sessions.tabs[0].id;
    assert_eq!(state.backend_commands.pending_count(), 2);

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 0);
    assert_eq!(state.sessions.tab_count(), 0);
    assert_eq!(state.terminal.tab_count(), 0);
    assert!(state.backend_commands.is_empty());
}

#[test]
fn close_pending_remote_command_tab_finishes_history_without_exit_code() {
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
    state.storage.command_history[0].started_at_unix_secs = 1;

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 0);
    assert_eq!(state.sessions.tab_count(), 0);
    assert_eq!(state.terminal.tab_count(), 0);
    assert!(state.backend_commands.is_empty());
    assert_eq!(state.storage.command_history[0].exit_code, None);
    assert!(state.storage.command_history[0].duration_ms.is_some());
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
fn close_pending_sftp_tab_cancels_queued_transfer_and_removes_commands() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let session_id = state.sessions.tabs[0].id;
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });
    state.apply(Message::UploadSftp { host_id });
    assert_eq!(state.backend_commands.pending_count(), 3);
    assert!(matches!(
        state.sessions.transfers[0].status,
        crate::model::TransferStatus::Queued
    ));

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 0);
    assert_eq!(state.sessions.tab_count(), 0);
    assert_eq!(state.sessions.sftp_browser_count(), 0);
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        state.sessions.transfers[0].status,
        crate::model::TransferStatus::Cancelled
    ));
}

#[test]
fn close_pending_sftp_tab_keeps_same_id_transfer_from_other_session() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let session_id = state.sessions.tabs[0].id;
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });
    state.apply(Message::UploadSftp { host_id });
    let transfer_id = state.sessions.transfers[0].id;
    let stale_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let mut stale_transfer = state.sessions.transfers[0].clone();
    stale_transfer.session_id = stale_session_id;
    stale_transfer.local_path = "C:/tmp/stale-app.tar.gz".to_owned();
    state.sessions.transfers.push(stale_transfer);

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert!(state.backend_commands.is_empty());
    assert!(matches!(
        state.sessions.transfers[0].status,
        crate::model::TransferStatus::Cancelled
    ));
    assert_eq!(state.sessions.transfers[1].id, transfer_id);
    assert_eq!(state.sessions.transfers[1].session_id, stale_session_id);
    assert!(matches!(
        state.sessions.transfers[1].status,
        crate::model::TransferStatus::Queued
    ));
}

#[test]
fn close_pending_tunnel_tab_removes_launch_commands_without_stop() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let rule = TunnelRule {
        name: "local-db".to_owned(),
        kind: TunnelKind::Local,
        bind_host: "127.0.0.1".to_owned(),
        bind_port: 15432,
        target_host: "10.0.0.5".to_owned(),
        target_port: 5432,
        auto_start: false,
    };
    state.storage.upsert_host(host);
    state.apply(Message::StartTunnel { host_id, rule });
    let session_id = state.sessions.tabs[0].id;
    assert_eq!(state.backend_commands.pending_count(), 2);
    assert!(matches!(
        state.sessions.tunnels[0].status,
        TunnelStatus::Starting
    ));

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert_eq!(outcome.queued_backend_commands, 0);
    assert_eq!(state.sessions.tab_count(), 0);
    assert_eq!(state.sessions.tunnel_runtime_count(), 0);
    assert!(state.backend_commands.is_empty());
}

#[test]
fn close_starting_tunnel_without_pending_launch_commands_requires_stop() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let rule = TunnelRule {
        name: "local-db".to_owned(),
        kind: TunnelKind::Local,
        bind_host: "127.0.0.1".to_owned(),
        bind_port: 15432,
        target_host: "10.0.0.5".to_owned(),
        target_port: 5432,
        auto_start: false,
    };
    state.storage.upsert_host(host);
    state.apply(Message::StartTunnel { host_id, rule });
    let session_id = state.sessions.tabs[0].id;
    state.backend_commands.drain();

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.sessions.tunnel_runtime_count(), 1);
    assert!(state.backend_commands.is_empty());
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
fn close_stale_sftp_tab_keeps_current_browser_owner() {
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
    assert_eq!(state.sessions.sftp_browsers[0].session_id, second_id);
    assert_eq!(state.sessions.sftp_browsers[0].current_dir, "/var/log");
}

#[test]
fn close_current_sftp_tab_reassigns_browser_owner() {
    let mut state = AppState::default();
    let first_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let second_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state.sessions.open_sftp_tab(first_id, host_id, "/home/ops");
    state.sessions.open_sftp_tab(second_id, host_id, "/var/log");

    let outcome = state.apply(Message::CloseSessionTab {
        session_id: second_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.sessions.sftp_browser_count(), 1);
    assert_eq!(state.sessions.sftp_browsers[0].session_id, first_id);
    assert!(
        state
            .sessions
            .set_sftp_entries_for_session(first_id, "/home/ops", Vec::new())
    );
}

#[test]
fn close_current_pending_sftp_tab_reassigns_browser_without_loading() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    let first_id = state.sessions.tabs[0].id;
    state.backend_commands.drain();
    state
        .sessions
        .set_status(first_id, SessionStatus::Connected);
    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/var/log".to_owned(),
    });
    let second_id = state.sessions.tabs[1].id;
    assert!(state.sessions.sftp_browsers[0].loading);

    let outcome = state.apply(Message::CloseSessionTab {
        session_id: second_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.sessions.sftp_browser_count(), 1);
    assert_eq!(state.sessions.sftp_browsers[0].session_id, first_id);
    assert!(!state.sessions.sftp_browsers[0].loading);
    assert!(state.backend_commands.is_empty());
}

#[test]
fn close_current_sftp_tab_reassigns_browser_to_available_session() {
    let mut state = AppState::default();
    let connected_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let disconnected_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let current_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state
        .sessions
        .open_sftp_tab(connected_id, host_id, "/home/ops");
    state
        .sessions
        .set_status(connected_id, SessionStatus::Connected);
    state
        .sessions
        .open_sftp_tab(disconnected_id, host_id, "/tmp");
    state
        .sessions
        .set_status(disconnected_id, SessionStatus::Disconnected);
    state
        .sessions
        .open_sftp_tab(current_id, host_id, "/var/log");

    let outcome = state.apply(Message::CloseSessionTab {
        session_id: current_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.sftp_browsers[0].session_id, connected_id);
    assert_eq!(state.sessions.tab_count(), 2);
}

#[test]
fn close_current_sftp_tab_removes_browser_when_only_disconnected_tabs_remain() {
    let mut state = AppState::default();
    let disconnected_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let current_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state
        .sessions
        .open_sftp_tab(disconnected_id, host_id, "/tmp");
    state
        .sessions
        .set_status(disconnected_id, SessionStatus::Disconnected);
    state
        .sessions
        .open_sftp_tab(current_id, host_id, "/var/log");

    let outcome = state.apply(Message::CloseSessionTab {
        session_id: current_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.sessions.sftp_browser_count(), 0);
}

#[test]
fn close_session_tab_message_reports_missing_tab() {
    let mut state = AppState::default();
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.state_changed);
    assert!(outcome.error.is_some());
    assert_eq!(state.ui.last_error.as_deref(), outcome.error.as_deref());
    assert!(state.backend_commands.is_empty());

    let dismiss_outcome = state.apply(Message::DismissUiError);

    assert!(dismiss_outcome.changed());
    assert!(state.ui.last_error.is_none());
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
        session_id,
        rule_name: "local-db".to_owned(),
        host_id: Some(host_id),
        status: TunnelStatus::Running,
        started_at_unix_secs: Some(1),
        last_error: None,
    });

    let outcome = state.apply(Message::CloseSessionTab { session_id });

    assert!(outcome.state_changed);
    assert!(outcome.error.is_some());
    assert_eq!(state.ui.last_error.as_deref(), outcome.error.as_deref());
    assert_eq!(state.sessions.tab_count(), 1);
    assert_eq!(state.sessions.tunnel_runtime_count(), 1);
    assert!(state.backend_commands.is_empty());
}

#[test]
fn close_tunnel_tab_removes_only_matching_session_runtime() {
    let mut state = AppState::default();
    let closed_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let current_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
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
    state
        .sessions
        .open_tunnel_tab(closed_session_id, host_id, &rule);
    state.sessions.tunnels.push(TunnelRuntimeState {
        session_id: closed_session_id,
        rule_name: "local-db".to_owned(),
        host_id: Some(host_id),
        status: TunnelStatus::Stopped,
        started_at_unix_secs: None,
        last_error: None,
    });
    state.sessions.tunnels.push(TunnelRuntimeState {
        session_id: current_session_id,
        rule_name: "local-db".to_owned(),
        host_id: Some(host_id),
        status: TunnelStatus::Stopped,
        started_at_unix_secs: None,
        last_error: None,
    });

    let outcome = state.apply(Message::CloseSessionTab {
        session_id: closed_session_id,
    });

    assert!(outcome.changed());
    assert_eq!(state.sessions.tunnel_runtime_count(), 1);
    assert_eq!(state.sessions.tunnels[0].session_id, current_session_id);
    assert!(matches!(
        state.sessions.tunnels[0].status,
        TunnelStatus::Stopped
    ));
}

#[test]
fn close_tunnel_tab_ignores_other_session_running_same_rule() {
    let mut state = AppState::default();
    let closed_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let current_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
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
    state
        .sessions
        .open_tunnel_tab(closed_session_id, host_id, &rule);
    state.sessions.tunnels.push(TunnelRuntimeState {
        session_id: closed_session_id,
        rule_name: "local-db".to_owned(),
        host_id: Some(host_id),
        status: TunnelStatus::Stopped,
        started_at_unix_secs: None,
        last_error: None,
    });
    state.sessions.tunnels.push(TunnelRuntimeState {
        session_id: current_session_id,
        rule_name: "local-db".to_owned(),
        host_id: Some(host_id),
        status: TunnelStatus::Running,
        started_at_unix_secs: Some(10),
        last_error: None,
    });

    let outcome = state.apply(Message::CloseSessionTab {
        session_id: closed_session_id,
    });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert_eq!(state.sessions.tunnel_runtime_count(), 1);
    assert_eq!(state.sessions.tunnels[0].session_id, current_session_id);
    assert!(matches!(
        state.sessions.tunnels[0].status,
        TunnelStatus::Running
    ));
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
        .sessions
        .set_status(session_id, SessionStatus::Connected);
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
fn send_remote_terminal_input_rejects_empty_command() {
    let mut state = AppState::default();
    let session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state
        .sessions
        .open_shell_tab(session_id, host_id, "production");
    state
        .sessions
        .set_status(session_id, SessionStatus::Connected);
    state
        .terminal
        .open_tab(crate::terminal::TerminalTabState::new(
            session_id,
            "production",
        ));
    state.ui.set_terminal_input(session_id, "  ");

    let outcome = state.apply(Message::SendTerminalInput { session_id });

    assert!(outcome.changed());
    assert!(outcome.error.as_deref().unwrap_or("").contains("不能为空"));
    assert!(state.backend_commands.is_empty());
    assert_eq!(state.storage.command_history_count(), 0);
}

#[test]
fn send_terminal_input_rejects_disconnected_or_failed_shell() {
    let mut state = AppState::default();
    let disconnected_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let failed_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let host_id = crate::model::HostId(uuid::Uuid::new_v4());
    state
        .sessions
        .open_shell_tab(disconnected_id, host_id, "disconnected");
    state.sessions.open_shell_tab(failed_id, host_id, "failed");
    assert!(
        state
            .sessions
            .set_status(disconnected_id, SessionStatus::Disconnected)
    );
    assert!(state.sessions.set_status(
        failed_id,
        SessionStatus::Failed {
            reason: "network".to_owned(),
        },
    ));
    state.ui.set_terminal_input(disconnected_id, "ls");
    state.ui.set_terminal_input(failed_id, "pwd");

    let disconnected = state.apply(Message::SendTerminalInput {
        session_id: disconnected_id,
    });
    let failed = state.apply(Message::SendTerminalInput {
        session_id: failed_id,
    });

    assert!(disconnected.changed());
    assert!(failed.changed());
    assert!(
        disconnected
            .error
            .as_deref()
            .unwrap_or("")
            .contains("不可交互")
    );
    assert!(failed.error.as_deref().unwrap_or("").contains("不可交互"));
    assert!(state.backend_commands.is_empty());
    assert_eq!(state.storage.command_history_count(), 0);
    assert_eq!(state.ui.terminal_input_for(disconnected_id), "ls");
    assert_eq!(state.ui.terminal_input_for(failed_id), "pwd");
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

#[test]
fn select_sftp_entry_reassigns_disconnected_browser_owner() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    let fallback_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    let disconnected_session_id = crate::model::SessionId(uuid::Uuid::new_v4());
    state.storage.upsert_host(host);
    state
        .sessions
        .open_sftp_tab(fallback_session_id, host_id, "/home/ops");
    state
        .sessions
        .set_status(fallback_session_id, SessionStatus::Connected);
    state
        .sessions
        .open_sftp_tab(disconnected_session_id, host_id, "/var/log");
    state
        .sessions
        .set_status(disconnected_session_id, SessionStatus::Disconnected);

    let outcome = state.apply(Message::SelectSftpEntry {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });

    assert!(outcome.changed());
    assert!(outcome.error.is_none());
    assert_eq!(
        state.sessions.sftp_browsers[0].session_id,
        fallback_session_id
    );
    assert_eq!(
        state.sessions.sftp_browsers[0].selected_path.as_deref(),
        Some("/home/ops/deploy.sh")
    );
    assert!(state.backend_commands.is_empty());
}

#[test]
fn select_sftp_entry_rejects_disconnected_browser_without_fallback_session() {
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
        .set_status(session_id, SessionStatus::Disconnected);

    let outcome = state.apply(Message::SelectSftpEntry {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });

    assert!(outcome.changed());
    assert!(
        outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("没有可用的 SFTP 会话")
    );
    assert!(state.sessions.sftp_browsers[0].selected_path.is_none());
    assert!(state.backend_commands.is_empty());
}
