use super::*;

#[test]
fn create_and_remove_sftp_actions_queue_path_requests() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();
    state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::NewDirName,
        value: "incoming".to_owned(),
    });

    let mkdir_outcome = state.apply(Message::CreateSftpDir { host_id });
    assert!(mkdir_outcome.changed());
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::Sftp { request, .. })
            if request.remote_path() == "/home/ops/incoming"
    ));

    state.backend_commands.drain();
    let remove_outcome = state.apply(Message::RemoveSftpFile {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });

    assert!(remove_outcome.changed());
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::Sftp { request, .. })
            if request.remote_path() == "/home/ops/deploy.sh"
    ));
}
