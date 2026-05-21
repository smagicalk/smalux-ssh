use super::*;

#[test]
fn local_terminal_input_reaches_backend_and_updates_terminal_buffer() {
    let mut state = AppState::default();
    let session_id = LOCAL_TERMINAL_SESSION_ID;
    state.apply(Message::UpdateTerminalInputDraft {
        session_id,
        input: "echo smagicalssh-visible".to_owned(),
    });
    state.apply(Message::SendTerminalInput { session_id });

    let mut executor = ScriptedBackendExecutor::new();
    executor.push_response(ScriptedBackendResponse::new(
        BackendCommandKind::SendShellInput,
        vec![BackendEvent::Output {
            session_id,
            line: "smagicalssh-visible".to_owned(),
        }],
    ));

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(state.ui.terminal_input_for(session_id), "");
    assert_eq!(
        state
            .terminal
            .tabs
            .iter()
            .find(|tab| tab.session_id == session_id)
            .map(|tab| tab.buffer.as_slice()),
        Some(
            [
                format!(
                    "{} echo smagicalssh-visible",
                    crate::backend::LocalShellProfile::default_for_platform().prompt
                ),
                "smagicalssh-visible".to_owned(),
            ]
            .as_slice()
        )
    );
}

#[test]
fn local_terminal_send_clears_input_immediately_without_waiting_for_pump() {
    let mut state = AppState::default();
    let session_id = LOCAL_TERMINAL_SESSION_ID;

    state.apply(Message::UpdateTerminalInputDraft {
        session_id,
        input: "pwd".to_owned(),
    });
    let outcome = state.apply(Message::SendTerminalInput { session_id });

    assert!(outcome.changed());
    assert_eq!(state.ui.terminal_input_for(session_id), "");
    assert_eq!(state.backend_commands.pending_count(), 1);
}
