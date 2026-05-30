use std::time::{Duration, Instant};

use super::*;
use smagical_backend_core::PtyRequest;
use smagical_core::SessionId;
use smagical_terminal::TerminalSize;
use uuid::Uuid;

fn session_id() -> SessionId {
    SessionId(Uuid::new_v4())
}

#[test]
fn drain_missing_local_session_returns_no_events() {
    let mut executor = LocalPtyBackendExecutor::default();
    let session_id = session_id();

    let events = executor
        .execute(BackendCommand::DrainSessionOutput { session_id })
        .expect("draining a missing local session should be a no-op");

    assert!(events.is_empty());
    assert_eq!(executor.session_count(), 0);
}

#[test]
fn disconnect_missing_local_session_still_reports_disconnected() {
    let mut executor = LocalPtyBackendExecutor::default();
    let session_id = session_id();

    let events = executor
        .execute(BackendCommand::Disconnect { session_id })
        .expect("disconnecting a missing local session should be idempotent");

    assert_eq!(events, vec![BackendEvent::Disconnected { session_id }]);
    assert_eq!(executor.session_count(), 0);
}

#[test]
fn local_pty_rejects_unsupported_commands_without_starting_session() {
    let mut executor = LocalPtyBackendExecutor::default();
    let session_id = session_id();

    let error = executor
        .execute(BackendCommand::RunCommand {
            session_id,
            request: smagical_backend_core::RemoteCommandRequest {
                command: "whoami".to_owned(),
                pty: None,
            },
        })
        .expect_err("local pty should reject remote command execution");

    assert!(matches!(
        error,
        BackendExecutionError::ChannelFailed {
            operation,
            reason,
        } if operation == "local pty" && reason.contains("RunCommand")
    ));
    assert_eq!(executor.session_count(), 0);
}

#[test]
#[ignore = "requires interactive Windows ConPTY output in the test runner"]
fn local_pty_starts_shell_and_accepts_input() {
    let mut executor = LocalPtyBackendExecutor::default();
    let session_id = session_id();

    let open = executor
        .execute(BackendCommand::OpenLocalShell {
            session_id,
            pty: PtyRequest::xterm(TerminalSize::default()),
        })
        .expect("local pty should open");
    let open_drain = executor
        .sessions
        .get_mut(&session_id)
        .expect("local session should exist")
        .drain_output_for(Duration::from_millis(750));
    let child_status = executor
        .sessions
        .get_mut(&session_id)
        .expect("local session should exist")
        .child_is_running();
    let output = executor
        .execute(BackendCommand::SendShellInput {
            session_id,
            input: local_echo_command("smagicalssh-pty"),
        })
        .expect("local pty should accept input");
    std::thread::sleep(fallback::LOCAL_PTY_FALLBACK_AFTER + Duration::from_millis(50));
    let delayed_start = executor
        .execute(BackendCommand::DrainSessionOutput { session_id })
        .expect("local pty should drain delayed output");
    let delayed_output = executor
        .sessions
        .get_mut(&session_id)
        .expect("local session should exist")
        .drain_output_for(Duration::from_secs(3));

    assert_eq!(executor.session_count(), 1);
    assert!(open.iter().any(|event| matches!(
        event,
        BackendEvent::ShellOpened { session_id: id } if *id == session_id
    )));
    assert!(
        child_status,
        "local shell exited before input; open={open:?}, open_drain={open_drain:?}"
    );
    assert!(
        output
            .iter()
            .chain(delayed_start.iter())
            .chain(delayed_output.iter())
            .any(|event| matches!(
                event,
                BackendEvent::Output { line, .. } if line.contains("smagicalssh-pty")
            )),
        "local pty output should contain echo marker after drain: open={open:?}, open_drain={open_drain:?}, output={output:?}, delayed_start={delayed_start:?}, delayed={delayed_output:?}"
    );
}

fn local_echo_command(text: &str) -> String {
    if cfg!(windows) {
        format!("Write-Output '{text}'\n")
    } else {
        format!("printf '{text}\\n'\n")
    }
}

#[test]
#[ignore = "requires interactive Windows ConPTY output in the test runner"]
fn due_fallback_starts_worker_without_blocking_drain() {
    let mut session =
        session::LocalPtySession::spawn(session_id(), &LocalShellProfile::default_for_platform())
            .expect("local pty should open");
    session.remember_fallback(local_echo_command("smagicalssh-fallback-worker"));

    std::thread::sleep(fallback::LOCAL_PTY_FALLBACK_AFTER + Duration::from_millis(20));
    let started_at = Instant::now();
    let first_drain = session.drain_output();

    assert!(
        started_at.elapsed() < Duration::from_millis(120),
        "starting fallback worker should not wait for command output"
    );
    assert!(first_drain.is_empty());

    let output = session.drain_output_for(Duration::from_secs(3));

    assert!(output.iter().any(|event| matches!(
        event,
        BackendEvent::Output { line, .. } if line.contains("smagicalssh-fallback-worker")
    )));
}
