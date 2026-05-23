use super::super::{HostId, UiState};
use super::{SftpActionDraft, SftpActionDraftField};
use uuid::Uuid;

#[test]
fn sftp_action_draft_starts_empty() {
    let draft = SftpActionDraft::new(HostId(Uuid::new_v4()));

    assert!(draft.local_path.is_empty());
    assert!(draft.remote_name.is_empty());
    assert!(draft.new_dir_name.is_empty());
}

#[test]
fn ui_state_sftp_action_messages_update_form_only() {
    let mut state = UiState::default();
    let host_id = HostId(Uuid::new_v4());

    state.set_sftp_action_field(
        host_id,
        SftpActionDraftField::LocalPath,
        "C:/tmp/app.tar.gz",
    );
    state.set_sftp_action_field(host_id, SftpActionDraftField::RemoteName, "app.tar.gz");
    state.set_sftp_action_field(host_id, SftpActionDraftField::NewDirName, "releases");

    assert_eq!(state.sftp_local_path_for(host_id), "C:/tmp/app.tar.gz");
    assert_eq!(state.sftp_remote_name_for(host_id), "app.tar.gz");
    assert_eq!(state.sftp_new_dir_name_for(host_id), "releases");
}
