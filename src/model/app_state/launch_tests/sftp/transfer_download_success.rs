use super::*;

#[test]
fn download_sftp_message_queues_transfer_and_download_request() {
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
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/deploy.sh".to_owned(),
    });

    let outcome = state.apply(Message::DownloadSftp {
        host_id,
        remote_path: "/home/ops/deploy.sh".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert_eq!(state.sessions.transfer_count(), 1);
    assert!(matches!(
        &state.sessions.transfers[0].direction,
        crate::model::TransferDirection::Download
    ));
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::Sftp { request, .. })
            if request.remote_path() == "/home/ops/deploy.sh"
    ));
}
