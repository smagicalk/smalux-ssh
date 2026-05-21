use super::*;
use crate::backend::{
    BackendAuth, ConnectionTarget, PtyRequest, RemoteCommandRequest, SftpRequest,
};
use crate::model::HostKeyVerification;
use crate::model::{HostId, KeyAlgorithm, SecretRef};
use crate::security::MemorySecretStore;
use crate::terminal::TerminalSize;
use smagical_ssh_client_core::channel_failure_parts;
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
        known_hosts: Vec::new(),
    }
}

fn assert_channel_failure(
    error: &BackendExecutionError,
    expected_operation: &str,
    expected_reason: &str,
) {
    let (operation, reason) = channel_failure_parts(error).expect("错误应该是 SSH channel 失败");
    assert_eq!(operation, expected_operation);
    assert_eq!(reason, expected_reason);
}

#[test]
fn executor_starts_with_empty_runtime_state() {
    let executor =
        RusshBackendExecutor::new(MemorySecretStore::new()).expect("执行器应该可以创建 runtime");

    assert_eq!(executor.connection_count(), 0);
    assert_eq!(executor.shell_count(), 0);
    assert_eq!(executor.sftp_count(), 0);
    assert_eq!(executor.tunnel_count(), 0);
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
fn host_key_rejected_error_is_connection_scoped() {
    let error = BackendExecutionError::HostKeyRejected {
        host: "example.com".to_owned(),
        port: 22,
        key_algorithm: KeyAlgorithm::Ed25519,
        fingerprint: "SHA256:new".to_owned(),
        verification: HostKeyVerification::Unknown,
    };

    assert_eq!(
        error.to_string(),
        "主机密钥未被信任：example.com:22 SHA256:new"
    );
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

    assert_channel_failure(&error, "sftp", "session is not connected");
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

    assert_channel_failure(&error, "start tunnel", "session is not connected");
}

#[test]
fn stop_tunnel_without_runtime_is_idempotent() {
    let mut executor =
        RusshBackendExecutor::new(MemorySecretStore::new()).expect("执行器应该可以创建 runtime");
    let session_id = session_id();

    let events = executor
        .execute(BackendCommand::StopTunnel {
            session_id,
            request: crate::backend::TunnelStopRequest::by_name("proxy"),
        })
        .expect("停止缺失隧道应该保持幂等");

    assert_eq!(
        events,
        vec![BackendEvent::TunnelStatusChanged {
            session_id,
            rule_name: "proxy".to_owned(),
            status: crate::model::TunnelStatus::Stopped,
        }]
    );
    assert_eq!(executor.tunnel_count(), 0);
}
