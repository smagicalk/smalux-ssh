use super::*;

#[test]
fn download_sftp_rejects_empty_and_root_remote_paths() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();

    let empty_outcome = state.apply(Message::DownloadSftp {
        host_id,
        remote_path: "  ".to_owned(),
    });
    let root_outcome = state.apply(Message::DownloadSftp {
        host_id,
        remote_path: " / ".to_owned(),
    });

    assert!(empty_outcome.changed());
    assert!(root_outcome.changed());
    assert!(
        empty_outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("不能为空")
    );
    assert!(
        root_outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("根目录")
    );
    assert_eq!(state.sessions.transfer_count(), 0);
    assert!(state.backend_commands.is_empty());
}
