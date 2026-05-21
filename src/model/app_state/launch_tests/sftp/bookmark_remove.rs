use super::*;

#[test]
fn remove_sftp_bookmark_updates_storage_or_reports_missing() {
    let mut state = AppState::default();
    let host_id = HostId(uuid::Uuid::new_v4());
    state
        .storage
        .upsert_sftp_bookmark(crate::model::SftpBookmark {
            host_id,
            label: "logs".to_owned(),
            remote_path: "/var/log".to_owned(),
        });

    let remove_outcome = state.apply(Message::RemoveSftpBookmark {
        host_id,
        remote_path: "/var/log".to_owned(),
    });
    let missing_outcome = state.apply(Message::RemoveSftpBookmark {
        host_id,
        remote_path: "/var/log".to_owned(),
    });

    assert!(remove_outcome.changed());
    assert_eq!(state.storage.sftp_bookmark_count(), 0);
    assert!(missing_outcome.changed());
    assert!(missing_outcome.error.is_some());
}
