use super::*;

#[test]
fn upload_sftp_message_queues_transfer_and_upload_request() {
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
        field: crate::model::SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });
    state.apply_message(Message::UpdateSftpActionDraft {
        host_id,
        field: crate::model::SftpActionDraftField::RemoteName,
        value: "release.tar.gz".to_owned(),
    });

    let outcome = state.apply_message(Message::UploadSftp { host_id });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert_eq!(state.core.sessions.transfer_count(), 1);
    assert!(matches!(
        &state.core.sessions.transfers[0].direction,
        crate::model::TransferDirection::Upload
    ));
    assert!(matches!(
        state.core.backend_commands.front(),
        Some(BackendCommand::Sftp { request, .. })
            if request.remote_path() == "/home/ops/release.tar.gz"
    ));
}
