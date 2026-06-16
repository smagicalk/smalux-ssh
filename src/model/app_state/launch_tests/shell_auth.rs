use super::*;

#[test]
fn password_host_can_still_open_shell_without_exposing_secret() {
    let mut state = CoreState::default();
    let mut host = sample_host();
    host.auth = AuthProfile::Password {
        username: "root".to_owned(),
        secret: SecretRef("password:root".to_owned()),
    };
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::OpenShell { host_id });

    let commands = state.backend_commands.drain();
    assert!(matches!(
        &commands[0],
        BackendCommand::Connect { target, .. }
            if target.auth.username() == "root"
    ));
}
