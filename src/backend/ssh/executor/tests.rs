use super::*;
use crate::backend::{BackendAuth, SftpRequest};
use crate::model::{HostId, SecretRef};
use crate::security::MemorySecretStore;
use crate::terminal::TerminalSize;
use uuid::Uuid;

fn session_id() -> SessionId {
    SessionId(Uuid::new_v4())
}

fn target(auth: BackendAuth) -> ConnectionTarget {
    ConnectionTarget {
        host_id: HostId(Uuid::new_v4()),
        address: "example.com".to_owned(),
        port: 22,
        auth,
    }
}

#[test]
fn executor_starts_with_empty_runtime_state() {
    let executor =
        RusshBackendExecutor::new(MemorySecretStore::new()).expect("执行器应该可以创建 runtime");

    assert_eq!(executor.connection_count(), 0);
    assert_eq!(executor.shell_count(), 0);
    assert_eq!(executor.sftp_count(), 0);
}

#[test]
fn connect_missing_secret_fails_before_network_access() {
    let mut executor =
        RusshBackendExecutor::new(MemorySecretStore::new()).expect("执行器应该可以创建 runtime");
    let session_id = session_id();

    let error = executor
        .execute(BackendCommand::Connect {
            session_id,
            target: target(BackendAuth::Password {
                username: "deploy".to_owned(),
                secret: SecretRef("missing".to_owned()),
            }),
        })
        .expect_err("缺失凭据应该在联网前失败");

    assert!(matches!(
        error,
        BackendExecutionError::AuthenticationFailed {
            username,
            reason,
        } if username == "deploy" && reason.contains("找不到凭据引用")
    ));
    assert_eq!(executor.connection_count(), 0);
}

#[test]
fn open_shell_requires_connected_session() {
    let mut executor =
        RusshBackendExecutor::new(MemorySecretStore::new()).expect("执行器应该可以创建 runtime");
    let session_id = session_id();

    let error = executor
        .execute(BackendCommand::OpenShell {
            session_id,
            pty: PtyRequest::xterm(TerminalSize::default()),
        })
        .expect_err("未连接会话不能打开 shell");

    assert!(matches!(
    error,
    BackendExecutionError::ChannelFailed {
        operation,
        reason,
    } if operation == "open shell" && reason == "session is not connected"
    ));
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

    assert!(matches!(
        error,
        BackendExecutionError::ChannelFailed {
            operation,
            reason,
        } if operation == "send shell input" && reason == "session is not connected"
    ));
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

    assert!(matches!(
        error,
        BackendExecutionError::ChannelFailed {
            operation,
            reason,
        } if operation == "run command" && reason == "session is not connected"
    ));
}

#[test]
fn disconnect_without_connection_still_emits_disconnected_event() {
    let mut executor =
        RusshBackendExecutor::new(MemorySecretStore::new()).expect("执行器应该可以创建 runtime");
    let session_id = session_id();

    let events = executor
        .execute(BackendCommand::Disconnect { session_id })
        .expect("断开缺失连接应该保持幂等");

    assert_eq!(events, vec![BackendEvent::Disconnected { session_id }]);
}

#[test]
fn sftp_requires_connected_session() {
    let mut executor =
        RusshBackendExecutor::new(MemorySecretStore::new()).expect("执行器应该可以创建 runtime");
    let session_id = session_id();

    let error = executor
        .execute(BackendCommand::Sftp {
            session_id,
            request: SftpRequest::ListDir {
                remote_path: "/".to_owned(),
            },
        })
        .expect_err("未连接会话不能打开 SFTP");

    assert!(matches!(
        error,
        BackendExecutionError::ChannelFailed {
            operation,
            reason,
        } if operation == "sftp" && reason == "session is not connected"
    ));
}

#[test]
fn start_tunnel_requires_connected_session() {
    let mut executor =
        RusshBackendExecutor::new(MemorySecretStore::new()).expect("执行器应该可以创建 runtime");
    let session_id = session_id();

    let error = executor
        .execute(BackendCommand::StartTunnel {
            session_id,
            request: crate::backend::TunnelStartRequest::new(crate::model::TunnelRule {
                name: "proxy".to_owned(),
                kind: crate::model::TunnelKind::Dynamic,
                bind_host: "127.0.0.1".to_owned(),
                bind_port: 1080,
                target_host: String::new(),
                target_port: 0,
                auto_start: false,
            })
            .expect("动态隧道请求应该有效"),
        })
        .expect_err("未连接会话不能启动隧道");

    assert!(matches!(
        error,
        BackendExecutionError::ChannelFailed {
            operation,
            reason,
        } if operation == "start tunnel" && reason == "session is not connected"
    ));
}
