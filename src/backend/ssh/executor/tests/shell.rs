use super::super::RusshBackendExecutor;
use super::common::{assert_channel_failure, session_id};
use crate::backend::{BackendCommand, BackendEvent, RemoteCommandRequest};
use crate::security::MemorySecretStore;
use crate::terminal::TerminalSize;
use smagical_backend_core::BackendExecutor;

#[test]
fn open_shell_requires_connected_session() {
    let mut executor =
        RusshBackendExecutor::new(MemorySecretStore::new()).expect("执行器应该可以创建 runtime");
    let session_id = session_id();

    let error = executor
        .execute(BackendCommand::OpenShell {
            session_id,
            pty: crate::backend::PtyRequest::xterm(TerminalSize::default()),
        })
        .expect_err("未连接会话不能打开 shell");

    assert_channel_failure(&error, "open shell", "session is not connected");
}

#[test]
fn send_shell_input_requires_connected_session() {
    let mut executor =
        RusshBackendExecutor::new(MemorySecretStore::new()).expect("执行器应该可以创建 runtime");
    let session_id = session_id();

    let error = executor
        .execute(BackendCommand::SendShellInput {
            session_id,
            input: "ls".to_owned(),
        })
        .expect_err("未连接会话不能发送 shell 输入");

    assert_channel_failure(&error, "send shell input", "session is not connected");
}

#[test]
fn drain_session_output_noops_for_remote_executor() {
    let mut executor =
        RusshBackendExecutor::new(MemorySecretStore::new()).expect("执行器应该可以创建 runtime");
    let session_id = session_id();

    let events = executor
        .execute(BackendCommand::DrainSessionOutput { session_id })
        .expect("远程执行器 drain 应保持幂等");

    assert!(events.is_empty());
}

#[test]
fn drain_session_output_without_shell_stays_empty_even_after_disconnect() {
    let mut executor =
        RusshBackendExecutor::new(MemorySecretStore::new()).expect("执行器应该可以创建 runtime");
    let session_id = session_id();

    let first = executor
        .execute(BackendCommand::DrainSessionOutput { session_id })
        .expect("缺失 shell 应保持幂等");
    let second = executor
        .execute(BackendCommand::Disconnect { session_id })
        .expect("断开缺失连接应保持幂等");

    assert!(first.is_empty());
    assert_eq!(second, vec![BackendEvent::Disconnected { session_id }]);
}

#[test]
fn run_command_requires_connected_session() {
    let mut executor =
        RusshBackendExecutor::new(MemorySecretStore::new()).expect("执行器应该可以创建 runtime");
    let session_id = session_id();

    let error = executor
        .execute(BackendCommand::RunCommand {
            session_id,
            request: RemoteCommandRequest::exec("uptime"),
        })
        .expect_err("未连接会话不能执行远程命令");

    assert_channel_failure(&error, "run command", "session is not connected");
}
