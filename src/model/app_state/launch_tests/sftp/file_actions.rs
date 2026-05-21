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

#[test]
fn remove_sftp_file_rejects_empty_and_root_paths() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.backend_commands.drain();

    let empty_outcome = state.apply(Message::RemoveSftpFile {
        host_id,
        remote_path: "  ".to_owned(),
    });
    let root_outcome = state.apply(Message::RemoveSftpFile {
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
    assert!(state.backend_commands.is_empty());
}

#[test]
fn create_sftp_dir_rejects_path_like_names() {
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
        value: "releases/2026".to_owned(),
    });

    let outcome = state.apply(Message::CreateSftpDir { host_id });

    assert!(outcome.changed());
    assert!(
        outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("路径分隔符")
    );
    assert!(state.backend_commands.is_empty());
}

#[test]
fn create_sftp_dir_rejects_parent_directory_alias() {
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
        value: "..".to_owned(),
    });

    let outcome = state.apply(Message::CreateSftpDir { host_id });

    assert!(outcome.changed());
    assert!(
        outcome
            .error
            .as_deref()
            .unwrap_or("")
            .contains("路径分隔符")
    );
    assert!(state.backend_commands.is_empty());
}
