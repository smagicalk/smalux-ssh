use super::*;

#[test]
fn remove_sftp_file_rejects_empty_and_root_paths() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    state.core.storage.upsert_host(host);

    state.apply_message(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.core.backend_commands.drain();

    let empty_outcome = state.apply_message(Message::RemoveSftpFile {
        host_id,
        remote_path: "  ".to_owned(),
    });
    let root_outcome = state.apply_message(Message::RemoveSftpFile {
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
    assert!(state.core.backend_commands.is_empty());
}
