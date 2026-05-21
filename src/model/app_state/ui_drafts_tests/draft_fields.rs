use super::*;

#[test]
fn command_draft_message_updates_ui_state_only() {
    let mut state = AppState::default();
    let host_id = HostId(Uuid::new_v4());

    let outcome = state.apply(Message::UpdateHostCommandDraft {
        host_id,
        command: "whoami".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(state.ui.remote_command_for(host_id), "whoami");
    assert_eq!(state.backend_commands.pending_count(), 0);
}

#[test]
fn sftp_initial_dir_draft_message_updates_ui_state_only() {
    let mut state = AppState::default();
    let host_id = HostId(Uuid::new_v4());

    let outcome = state.apply(Message::UpdateHostSftpInitialDirDraft {
        host_id,
        initial_dir: "/etc".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(state.ui.sftp_initial_dir_for(host_id), "/etc");
    assert_eq!(state.sessions.tab_count(), 0);
}

#[test]
fn sftp_action_draft_message_updates_ui_state_only() {
    let mut state = AppState::default();
    let host_id = HostId(Uuid::new_v4());

    let outcome = state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(state.ui.sftp_local_path_for(host_id), "C:/tmp/app.tar.gz");
    assert_eq!(state.backend_commands.pending_count(), 0);
}
