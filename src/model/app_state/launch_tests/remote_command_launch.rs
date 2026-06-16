use super::*;

#[test]
fn run_remote_command_can_request_pty() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    state.core.storage.upsert_host(host);

    state.apply_message(Message::RunRemoteCommand {
        host_id,
        command: "top".to_owned(),
        request_pty: true,
    });

    let commands = state.core.backend_commands.drain();

    assert!(matches!(
        &commands[1],
        BackendCommand::RunCommand { request, .. }
            if request.command == "top" && request.pty.is_some()
    ));
}
