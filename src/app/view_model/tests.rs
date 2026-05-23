use super::*;
use crate::model::{AppState, AuthProfile, Host, HostId, KeyAlgorithm, KnownHostEntry, SecretRef};
use uuid::Uuid;

#[test]
fn app_view_model_uses_local_terminal_when_no_tab_is_open() {
    let state = AppState::default();

    let vm = app_view_model(&state);

    assert_eq!(
        vm.terminal.title,
        crate::model::DEFAULT_LOCAL_TERMINAL_TITLE
    );
    assert_eq!(vm.terminal.status, "Ready");
    assert!(vm.terminal.can_send_input);
}

#[test]
fn auth_label_covers_password_without_secret_leakage() {
    let mut state = AppState::default();
    state.storage.upsert_host(Host {
        id: HostId(Uuid::new_v4()),
        name: "root".to_owned(),
        group_id: None,
        tags: Vec::new(),
        address: "example.com".to_owned(),
        port: 22,
        auth: AuthProfile::Password {
            username: "root".to_owned(),
            secret: SecretRef("password:root".to_owned()),
        },
        proxy: None,
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    });

    let vm = app_view_model(&state);

    assert_eq!(vm.hosts[0].auth, "Password");
}

#[test]
fn app_view_model_projects_known_hosts_for_tool_panel() {
    let mut state = AppState::default();
    state.storage.upsert_known_host(KnownHostEntry::untrusted(
        "example.com",
        22,
        KeyAlgorithm::Ed25519,
        "SHA256:new",
    ));

    let vm = app_view_model(&state);

    assert_eq!(vm.known_hosts.len(), 1);
    assert_eq!(vm.known_hosts[0].host, "example.com");
    assert_eq!(vm.known_hosts[0].port, 22);
    assert_eq!(vm.known_hosts[0].fingerprint, "SHA256:new");
    assert_eq!(vm.known_hosts[0].status, "pending");
}

#[test]
fn app_view_model_keeps_sftp_panel_on_active_host_without_browser() {
    let mut state = AppState::default();
    let sftp_host = Host {
        id: HostId(Uuid::new_v4()),
        name: "files".to_owned(),
        group_id: None,
        tags: Vec::new(),
        address: "files.example.com".to_owned(),
        port: 22,
        auth: AuthProfile::Agent {
            username: "deploy".to_owned(),
            key_hint: None,
        },
        proxy: None,
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    };
    let shell_host = Host {
        id: HostId(Uuid::new_v4()),
        name: "shell".to_owned(),
        group_id: None,
        tags: Vec::new(),
        address: "shell.example.com".to_owned(),
        port: 22,
        auth: AuthProfile::Agent {
            username: "deploy".to_owned(),
            key_hint: None,
        },
        proxy: None,
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    };
    let sftp_host_id = sftp_host.id;
    let shell_host_id = shell_host.id;
    state.storage.upsert_host(sftp_host);
    state.storage.upsert_host(shell_host);
    state.sessions.open_sftp_tab(
        crate::model::SessionId(Uuid::new_v4()),
        sftp_host_id,
        "/var/log",
    );
    state.sessions.open_shell_tab(
        crate::model::SessionId(Uuid::new_v4()),
        shell_host_id,
        "shell",
    );

    let vm = app_view_model(&state);

    assert_eq!(vm.sftp.host_id, shell_host_id.0.to_string());
    assert_eq!(vm.sftp.title, "SFTP · shell");
    assert_eq!(vm.sftp.current_dir, "/");
    assert!(vm.sftp.entries.is_empty());
}
