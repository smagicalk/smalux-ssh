use super::super::*;
use crate::model::{
    AgentSource, AuthProfile, Host, HostId, Message, SessionId, SessionKind, SplitAxis,
    WorkspaceState, WorkspaceTabSnapshot,
};
use crate::terminal::TerminalTabState;
use uuid::Uuid;

fn host() -> Host {
    Host {
        id: HostId(Uuid::new_v4()),
        name: "production".to_owned(),
        group_id: None,
        icon_key: "server".to_owned(),
        tags: Vec::new(),
        address: "prod.example.com".to_owned(),
        port: 22,
        auth: AuthProfile::Agent {
            username: "deploy".to_owned(),
            source: AgentSource::Auto,
            key_hint: None,
        },
        proxy: None,
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    }
}

#[test]
fn save_workspace_snapshot_records_current_tabs_and_layout() {
    let mut state = AppState::default();
    let host = host();
    let host_id = host.id;
    let session_id = SessionId(Uuid::new_v4());
    state.storage.upsert_host(host);
    state
        .sessions
        .open_shell_tab(session_id, host_id, "production");
    state
        .terminal
        .open_tab(TerminalTabState::new(session_id, "production"));

    let outcome = state.apply(Message::SaveWorkspaceSnapshot);

    assert!(outcome.changed());
    let workspace = state
        .storage
        .workspace
        .as_ref()
        .expect("应该保存工作区快照");
    assert_eq!(workspace.tabs.len(), 1);
    assert_eq!(workspace.active_tab, Some(session_id));
    assert!(workspace.layout.is_some());
}

#[test]
fn restore_workspace_snapshot_rebuilds_session_and_terminal_tabs() {
    let mut state = AppState::default();
    let host = host();
    let host_id = host.id;
    let shell_id = SessionId(Uuid::new_v4());
    let sftp_id = SessionId(Uuid::new_v4());
    let mut workspace = WorkspaceState::empty("restore");
    state.storage.upsert_host(host);
    workspace.upsert_tab(WorkspaceTabSnapshot {
        session_id: shell_id,
        host_id: Some(host_id),
        kind: SessionKind::Shell,
        title: "shell".to_owned(),
        working_directory: None,
    });
    workspace.upsert_tab(WorkspaceTabSnapshot {
        session_id: sftp_id,
        host_id: Some(host_id),
        kind: SessionKind::Sftp,
        title: "SFTP /home/ops".to_owned(),
        working_directory: Some("/home/ops".to_owned()),
    });
    workspace.active_tab = Some(shell_id);
    workspace.rebuild_linear_layout(SplitAxis::Horizontal);
    state.storage.save_workspace(workspace);

    let outcome = state.apply(Message::RestoreWorkspaceSnapshot);

    assert!(outcome.changed());
    assert_eq!(state.sessions.tab_count(), 2);
    assert_eq!(state.terminal.tab_count(), 1);
    assert_eq!(state.sessions.sftp_browser_count(), 1);
    assert_eq!(state.sessions.sftp_browsers[0].current_dir, "/home/ops");
    assert_eq!(state.sessions.active_tab, Some(shell_id));
    assert_eq!(state.terminal.active_tab, Some(shell_id));
}

#[test]
fn clear_workspace_snapshot_removes_saved_workspace() {
    let mut state = AppState::default();
    state
        .storage
        .save_workspace(WorkspaceState::empty("restore"));

    let outcome = state.apply(Message::ClearWorkspaceSnapshot);

    assert!(outcome.changed());
    assert!(state.storage.workspace.is_none());
}
