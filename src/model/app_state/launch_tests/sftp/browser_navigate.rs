use super::*;

#[test]
fn navigate_sftp_message_queues_target_directory_listing() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    state.core.storage.upsert_host(host);

    state.apply_message(Message::OpenSftp {
        host_id,
        initial_dir: "/home/ops".to_owned(),
    });
    state.core.backend_commands.drain();

    let outcome = state.apply_message(Message::NavigateSftp {
        host_id,
        remote_path: "/etc".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(outcome.queued_backend_commands, 1);
    assert!(matches!(
        state.core.backend_commands.front(),
        Some(BackendCommand::Sftp { request, .. })
            if request.remote_path() == "/etc"
    ));
}
