use super::*;

#[test]
fn create_and_remove_sftp_actions_queue_path_requests() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    state.core.storage.upsert_host(host);

    state.apply_message(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.core.backend_commands.drain();
    state.apply_message(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::NewDirName,
        value: "incoming".to_owned(),
    });

    let mkdir_outcome = state.apply_message(Message::CreateSftpDir { host_id });
    assert!(mkdir_outcome.changed());
    assert!(matches!(
        state.core.backend_commands.front(),
        Some(BackendCommand::Sftp { request, .. })
            if request.remote_path() == "/home/ops/incoming"
    ));

    state.core.backend_commands.drain();
    let remove_outcome = state.apply_message(Message::RemoveSftpFile {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });

    assert!(remove_outcome.changed());
    assert!(matches!(
        state.core.backend_commands.front(),
        Some(BackendCommand::Sftp { request, .. })
            if request.remote_path() == "/home/ops/deploy.sh"
    ));
}
