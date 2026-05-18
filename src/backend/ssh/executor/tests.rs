use super::*;
use crate::backend::{BackendAuth, SftpRequest};
use crate::model::HostKeyVerification;
use crate::model::{HostId, KeyAlgorithm, SecretRef};
use crate::security::MemorySecretStore;
use crate::terminal::TerminalSize;
use std::collections::HashMap;
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
fn taking_cached_session_resources_removes_only_target_session() {
    let target_session_id = session_id();
    let other_session_id = session_id();
    let mut shells = HashMap::from([
        (target_session_id, "target-shell"),
        (other_session_id, "other-shell"),
    ]);
    let mut sftps = HashMap::from([
        (target_session_id, "target-sftp"),
        (other_session_id, "other-sftp"),
    ]);
    let mut connections = HashMap::from([
        (target_session_id, "target-connection"),
        (other_session_id, "other-connection"),
    ]);

    let resources =
        take_cached_session_resources(&mut shells, &mut sftps, &mut connections, target_session_id);

    assert_eq!(
        resources,
        CachedSessionResources {
            shell: Some("target-shell"),
            sftp: Some("target-sftp"),
            connection: Some("target-connection"),
        }
    );
    assert!(!shells.contains_key(&target_session_id));
    assert!(!sftps.contains_key(&target_session_id));
    assert!(!connections.contains_key(&target_session_id));
    assert_eq!(shells.get(&other_session_id), Some(&"other-shell"));
    assert_eq!(sftps.get(&other_session_id), Some(&"other-sftp"));
    assert_eq!(
        connections.get(&other_session_id),
        Some(&"other-connection")
    );
}

#[test]
fn taking_cached_session_resources_is_idempotent_for_missing_session() {
    let missing_session_id = session_id();
    let other_session_id = session_id();
    let mut shells = HashMap::from([(other_session_id, "other-shell")]);
    let mut sftps = HashMap::from([(other_session_id, "other-sftp")]);
    let mut connections = HashMap::from([(other_session_id, "other-connection")]);

    let resources = take_cached_session_resources(
        &mut shells,
        &mut sftps,
        &mut connections,
        missing_session_id,
    );

    assert_eq!(
        resources,
        CachedSessionResources {
            shell: None::<&str>,
            sftp: None::<&str>,
            connection: None::<&str>,
        }
    );
    assert_eq!(shells.get(&other_session_id), Some(&"other-shell"));
    assert_eq!(sftps.get(&other_session_id), Some(&"other-sftp"));
    assert_eq!(
        connections.get(&other_session_id),
        Some(&"other-connection")
    );
}

#[test]
fn taking_cached_session_subresources_removes_only_target_session() {
    let target_session_id = session_id();
    let other_session_id = session_id();
    let mut shells = HashMap::from([
        (target_session_id, "target-shell"),
        (other_session_id, "other-shell"),
    ]);
    let mut sftps = HashMap::from([
        (target_session_id, "target-sftp"),
        (other_session_id, "other-sftp"),
    ]);

    let resources = take_cached_session_subresources(&mut shells, &mut sftps, target_session_id);

    assert_eq!(
        resources,
        CachedSessionSubresources {
            shell: Some("target-shell"),
            sftp: Some("target-sftp"),
        }
    );
    assert!(!shells.contains_key(&target_session_id));
    assert!(!sftps.contains_key(&target_session_id));
    assert_eq!(shells.get(&other_session_id), Some(&"other-shell"));
    assert_eq!(sftps.get(&other_session_id), Some(&"other-sftp"));
}

#[test]
fn taking_cached_session_subresources_is_idempotent_for_missing_session() {
    let missing_session_id = session_id();
    let other_session_id = session_id();
    let mut shells = HashMap::from([(other_session_id, "other-shell")]);
    let mut sftps = HashMap::from([(other_session_id, "other-sftp")]);

    let resources = take_cached_session_subresources(&mut shells, &mut sftps, missing_session_id);

    assert_eq!(
        resources,
        CachedSessionSubresources {
            shell: None::<&str>,
            sftp: None::<&str>,
        }
    );
    assert_eq!(shells.get(&other_session_id), Some(&"other-shell"));
    assert_eq!(sftps.get(&other_session_id), Some(&"other-sftp"));
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
fn shell_input_failure_drops_only_failed_cached_shell() {
    let failed_session_id = session_id();
    let other_session_id = session_id();
    let result: Result<(), BackendExecutionError> = Err(BackendExecutionError::ChannelFailed {
        operation: "shell input".to_owned(),
        reason: "channel closed".to_owned(),
    });
    let mut cached_shells = HashMap::from([
        (failed_session_id, "failed-shell"),
        (other_session_id, "other-shell"),
    ]);

    let dropped =
        drop_cached_shell_after_failed_input(&mut cached_shells, failed_session_id, &result);

    assert!(dropped);
    assert!(!cached_shells.contains_key(&failed_session_id));
    assert_eq!(cached_shells.get(&other_session_id), Some(&"other-shell"));
}

#[test]
fn shell_input_cache_survives_success_and_non_channel_failures() {
    let success_session_id = session_id();
    let sftp_failure_session_id = session_id();
    let success: Result<(), BackendExecutionError> = Ok(());
    let sftp_failure: Result<(), BackendExecutionError> = Err(BackendExecutionError::SftpFailed {
        operation: "list dir".to_owned(),
        reason: "permission denied".to_owned(),
    });
    let mut cached_shells = HashMap::from([
        (success_session_id, "success-shell"),
        (sftp_failure_session_id, "sftp-failure-shell"),
    ]);

    let dropped_after_success =
        drop_cached_shell_after_failed_input(&mut cached_shells, success_session_id, &success);
    let dropped_after_sftp_failure = drop_cached_shell_after_failed_input(
        &mut cached_shells,
        sftp_failure_session_id,
        &sftp_failure,
    );

    assert!(!dropped_after_success);
    assert!(!dropped_after_sftp_failure);
    assert_eq!(
        cached_shells.get(&success_session_id),
        Some(&"success-shell")
    );
    assert_eq!(
        cached_shells.get(&sftp_failure_session_id),
        Some(&"sftp-failure-shell")
    );
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
fn sftp_failure_drops_only_failed_cached_session() {
    let failed_session_id = session_id();
    let other_session_id = session_id();
    let result: Result<Vec<BackendEvent>, BackendExecutionError> =
        Err(BackendExecutionError::SftpFailed {
            operation: "list dir".to_owned(),
            reason: "permission denied".to_owned(),
        });
    let mut cached_sftps = HashMap::from([
        (failed_session_id, "failed-session"),
        (other_session_id, "other-session"),
    ]);

    let dropped =
        drop_cached_sftp_after_failed_request(&mut cached_sftps, failed_session_id, &result);

    assert!(dropped);
    assert!(!cached_sftps.contains_key(&failed_session_id));
    assert_eq!(cached_sftps.get(&other_session_id), Some(&"other-session"));
}

#[test]
fn sftp_cache_survives_success_and_non_sftp_failures() {
    let success_session_id = session_id();
    let channel_failure_session_id = session_id();
    let success: Result<Vec<BackendEvent>, BackendExecutionError> = Ok(Vec::new());
    let channel_failure: Result<Vec<BackendEvent>, BackendExecutionError> =
        Err(BackendExecutionError::ChannelFailed {
            operation: "read".to_owned(),
            reason: "channel closed".to_owned(),
        });
    let mut cached_sftps = HashMap::from([
        (success_session_id, "success-session"),
        (channel_failure_session_id, "channel-failure-session"),
    ]);

    let dropped_after_success =
        drop_cached_sftp_after_failed_request(&mut cached_sftps, success_session_id, &success);
    let dropped_after_channel_failure = drop_cached_sftp_after_failed_request(
        &mut cached_sftps,
        channel_failure_session_id,
        &channel_failure,
    );

    assert!(!dropped_after_success);
    assert!(!dropped_after_channel_failure);
    assert_eq!(
        cached_sftps.get(&success_session_id),
        Some(&"success-session")
    );
    assert_eq!(
        cached_sftps.get(&channel_failure_session_id),
        Some(&"channel-failure-session")
    );
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
