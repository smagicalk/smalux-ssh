use super::*;
use crate::backend::{BackendAuth, SftpRequest};
use crate::model::HostKeyVerification;
use crate::model::{HostId, KeyAlgorithm, SecretRef};
use crate::security::MemorySecretStore;
use crate::terminal::TerminalSize;
use smagical_ssh_client_core::{channel_failure_parts, channel_reason_error, sftp_error};
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
fn taking_cached_session_resources_detaches_all_target_resources_before_close() {
    let session_id = session_id();
    let mut shells = HashMap::from([(session_id, "shell")]);
    let mut sftps = HashMap::from([(session_id, "sftp")]);
    let mut connections = HashMap::from([(session_id, "connection")]);

    let resources =
        take_cached_session_resources(&mut shells, &mut sftps, &mut connections, session_id);

    assert_eq!(
        resources,
        CachedSessionResources {
            shell: Some("shell"),
            sftp: Some("sftp"),
            connection: Some("connection"),
        }
    );
    assert!(shells.is_empty());
    assert!(sftps.is_empty());
    assert!(connections.is_empty());
}

#[test]
fn taking_cached_session_runtime_resources_detaches_owned_tunnels() {
    let target_session_id = session_id();
    let other_session_id = session_id();
    let mut shells = HashMap::from([(target_session_id, "target-shell")]);
    let mut sftps = HashMap::from([(target_session_id, "target-sftp")]);
    let mut connections = HashMap::from([(target_session_id, "target-connection")]);
    let mut tunnels = HashMap::from([
        (
            "proxy".to_owned(),
            TestTunnel {
                session_id: target_session_id,
                rule_name: "proxy".to_owned(),
                stopped: false,
            },
        ),
        (
            "metrics".to_owned(),
            TestTunnel {
                session_id: other_session_id,
                rule_name: "metrics".to_owned(),
                stopped: false,
            },
        ),
    ]);

    let resources = take_cached_session_runtime_resources(
        &mut shells,
        &mut sftps,
        &mut connections,
        &mut tunnels,
        target_session_id,
    );

    assert_eq!(
        resources.cached_resources,
        CachedSessionResources {
            shell: Some("target-shell"),
            sftp: Some("target-sftp"),
            connection: Some("target-connection"),
        }
    );
    assert_eq!(resources.tunnels.len(), 1);
    assert_eq!(resources.tunnels[0].rule_name, "proxy");
    assert!(shells.is_empty());
    assert!(sftps.is_empty());
    assert!(connections.is_empty());
    assert!(!tunnels.contains_key("proxy"));
    assert_eq!(
        tunnels.get("metrics"),
        Some(&TestTunnel {
            session_id: other_session_id,
            rule_name: "metrics".to_owned(),
            stopped: false,
        })
    );
}

#[test]
fn taking_cached_session_runtime_resources_is_idempotent_for_missing_session() {
    let missing_session_id = session_id();
    let other_session_id = session_id();
    let mut shells = HashMap::from([(other_session_id, "other-shell")]);
    let mut sftps = HashMap::from([(other_session_id, "other-sftp")]);
    let mut connections = HashMap::from([(other_session_id, "other-connection")]);
    let mut tunnels = HashMap::from([(
        "metrics".to_owned(),
        TestTunnel {
            session_id: other_session_id,
            rule_name: "metrics".to_owned(),
            stopped: false,
        },
    )]);

    let resources = take_cached_session_runtime_resources(
        &mut shells,
        &mut sftps,
        &mut connections,
        &mut tunnels,
        missing_session_id,
    );

    assert_eq!(
        resources.cached_resources,
        CachedSessionResources {
            shell: None::<&str>,
            sftp: None::<&str>,
            connection: None::<&str>,
        }
    );
    assert!(resources.tunnels.is_empty());
    assert_eq!(shells.get(&other_session_id), Some(&"other-shell"));
    assert_eq!(sftps.get(&other_session_id), Some(&"other-sftp"));
    assert_eq!(
        connections.get(&other_session_id),
        Some(&"other-connection")
    );
    assert_eq!(
        tunnels.get("metrics"),
        Some(&TestTunnel {
            session_id: other_session_id,
            rule_name: "metrics".to_owned(),
            stopped: false,
        })
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
fn replacing_cached_shell_returns_previous_shell_for_same_session() {
    let session_id = session_id();
    let mut shells = HashMap::from([(session_id, "old-shell")]);

    let previous = replace_cached_shell(&mut shells, session_id, "new-shell");

    assert_eq!(previous, Some("old-shell"));
    assert_eq!(shells.get(&session_id), Some(&"new-shell"));
}

#[test]
fn replacing_cached_shell_keeps_other_sessions() {
    let target_session_id = session_id();
    let other_session_id = session_id();
    let mut shells = HashMap::from([(other_session_id, "other-shell")]);

    let previous = replace_cached_shell(&mut shells, target_session_id, "target-shell");

    assert_eq!(previous, None);
    assert_eq!(shells.get(&target_session_id), Some(&"target-shell"));
    assert_eq!(shells.get(&other_session_id), Some(&"other-shell"));
}

#[test]
fn replacing_cached_sftp_returns_previous_sftp_for_same_session() {
    let session_id = session_id();
    let mut sftps = HashMap::from([(session_id, "old-sftp")]);

    let previous = replace_cached_sftp(&mut sftps, session_id, "new-sftp");

    assert_eq!(previous, Some("old-sftp"));
    assert_eq!(sftps.get(&session_id), Some(&"new-sftp"));
}

#[test]
fn replacing_cached_sftp_keeps_other_sessions() {
    let target_session_id = session_id();
    let other_session_id = session_id();
    let mut sftps = HashMap::from([(other_session_id, "other-sftp")]);

    let previous = replace_cached_sftp(&mut sftps, target_session_id, "target-sftp");

    assert_eq!(previous, None);
    assert_eq!(sftps.get(&target_session_id), Some(&"target-sftp"));
    assert_eq!(sftps.get(&other_session_id), Some(&"other-sftp"));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OwnedTunnel {
    session_id: SessionId,
}

impl TunnelOwner for OwnedTunnel {
    fn session_id(&self) -> SessionId {
        self.session_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TestTunnel {
    session_id: SessionId,
    rule_name: String,
    stopped: bool,
}

impl TunnelOwner for TestTunnel {
    fn session_id(&self) -> SessionId {
        self.session_id
    }
}

impl RuleNamedTunnel for TestTunnel {
    fn rule_name(&self) -> &str {
        &self.rule_name
    }
}

impl StoppableTunnel for TestTunnel {
    fn stop(&self) {
        STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow_mut().push(self.rule_name.clone()));
    }
}

thread_local! {
    static STOPPED_TEST_TUNNEL_NAMES: std::cell::RefCell<Vec<String>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

#[test]
fn removing_tunnel_requires_matching_session_and_rule() {
    let owner_session_id = session_id();
    let stale_session_id = session_id();
    let other_session_id = session_id();
    let mut tunnels = HashMap::from([
        (
            "proxy".to_owned(),
            OwnedTunnel {
                session_id: owner_session_id,
            },
        ),
        (
            "metrics".to_owned(),
            OwnedTunnel {
                session_id: other_session_id,
            },
        ),
    ]);

    let stale = remove_tunnel_for_session_rule(&mut tunnels, stale_session_id, "proxy");
    let missing = remove_tunnel_for_session_rule(&mut tunnels, owner_session_id, "missing");
    let removed = remove_tunnel_for_session_rule(&mut tunnels, owner_session_id, "proxy");

    assert_eq!(stale, None);
    assert_eq!(missing, None);
    assert_eq!(
        removed,
        Some(OwnedTunnel {
            session_id: owner_session_id,
        })
    );
    assert!(!tunnels.contains_key("proxy"));
    assert_eq!(
        tunnels.get("metrics"),
        Some(&OwnedTunnel {
            session_id: other_session_id,
        })
    );
}

#[test]
fn replacing_tunnel_stops_previous_same_rule() {
    let old_session_id = session_id();
    let new_session_id = session_id();
    let mut tunnels = HashMap::from([(
        "proxy".to_owned(),
        TestTunnel {
            session_id: old_session_id,
            rule_name: "proxy".to_owned(),
            stopped: false,
        },
    )]);
    STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow_mut().clear());

    replace_tunnel_stopping_previous(
        &mut tunnels,
        TestTunnel {
            session_id: new_session_id,
            rule_name: "proxy".to_owned(),
            stopped: false,
        },
    );

    assert_eq!(
        STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow().clone()),
        ["proxy"]
    );
    assert_eq!(
        tunnels.get("proxy"),
        Some(&TestTunnel {
            session_id: new_session_id,
            rule_name: "proxy".to_owned(),
            stopped: false,
        })
    );
}

#[test]
fn replacing_tunnel_keeps_unrelated_rules_running() {
    let existing_session_id = session_id();
    let new_session_id = session_id();
    let mut tunnels = HashMap::from([(
        "metrics".to_owned(),
        TestTunnel {
            session_id: existing_session_id,
            rule_name: "metrics".to_owned(),
            stopped: false,
        },
    )]);
    STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow_mut().clear());

    replace_tunnel_stopping_previous(
        &mut tunnels,
        TestTunnel {
            session_id: new_session_id,
            rule_name: "proxy".to_owned(),
            stopped: false,
        },
    );

    assert!(STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow().is_empty()));
    assert!(tunnels.contains_key("metrics"));
    assert_eq!(
        tunnels.get("proxy"),
        Some(&TestTunnel {
            session_id: new_session_id,
            rule_name: "proxy".to_owned(),
            stopped: false,
        })
    );
}

#[test]
fn taking_tunnels_for_session_removes_only_owned_tunnels() {
    let owner_session_id = session_id();
    let other_session_id = session_id();
    let mut tunnels = HashMap::from([
        (
            "proxy".to_owned(),
            TestTunnel {
                session_id: owner_session_id,
                rule_name: "proxy".to_owned(),
                stopped: false,
            },
        ),
        (
            "db".to_owned(),
            TestTunnel {
                session_id: owner_session_id,
                rule_name: "db".to_owned(),
                stopped: false,
            },
        ),
        (
            "metrics".to_owned(),
            TestTunnel {
                session_id: other_session_id,
                rule_name: "metrics".to_owned(),
                stopped: false,
            },
        ),
    ]);

    let removed = take_tunnels_for_session(&mut tunnels, owner_session_id);

    assert_eq!(removed.len(), 2);
    assert!(removed.iter().any(|tunnel| tunnel.rule_name == "proxy"));
    assert!(removed.iter().any(|tunnel| tunnel.rule_name == "db"));
    assert!(!tunnels.contains_key("proxy"));
    assert!(!tunnels.contains_key("db"));
    assert_eq!(
        tunnels.get("metrics"),
        Some(&TestTunnel {
            session_id: other_session_id,
            rule_name: "metrics".to_owned(),
            stopped: false,
        })
    );
}

#[test]
fn taking_tunnels_for_missing_session_keeps_all_tunnels() {
    let missing_session_id = session_id();
    let other_session_id = session_id();
    let mut tunnels = HashMap::from([(
        "metrics".to_owned(),
        TestTunnel {
            session_id: other_session_id,
            rule_name: "metrics".to_owned(),
            stopped: false,
        },
    )]);

    let removed = take_tunnels_for_session(&mut tunnels, missing_session_id);

    assert!(removed.is_empty());
    assert_eq!(
        tunnels.get("metrics"),
        Some(&TestTunnel {
            session_id: other_session_id,
            rule_name: "metrics".to_owned(),
            stopped: false,
        })
    );
}

#[test]
fn stopping_detached_tunnels_stops_each_removed_tunnel() {
    let session_id = session_id();
    let tunnels = vec![
        TestTunnel {
            session_id,
            rule_name: "proxy".to_owned(),
            stopped: false,
        },
        TestTunnel {
            session_id,
            rule_name: "db".to_owned(),
            stopped: false,
        },
    ];
    STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow_mut().clear());

    stop_detached_tunnels(session_id, tunnels, "test");

    let mut stopped = STOPPED_TEST_TUNNEL_NAMES.with(|names| names.borrow().clone());
    stopped.sort();
    assert_eq!(stopped, ["db", "proxy"]);
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
fn shell_input_failure_drops_only_failed_cached_shell() {
    let failed_session_id = session_id();
    let other_session_id = session_id();
    let result: Result<(), BackendExecutionError> =
        Err(channel_reason_error("shell input", "channel closed"));
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
    let sftp_failure: Result<(), BackendExecutionError> =
        Err(sftp_error("list dir", "permission denied"));
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
fn shell_input_drop_gate_is_strict_about_channel_failures_only() {
    let channel_failure: Result<(), BackendExecutionError> =
        Err(channel_reason_error("shell input", "channel closed"));
    let sftp_failure: Result<(), BackendExecutionError> =
        Err(sftp_error("list dir", "permission denied"));
    let success: Result<(), BackendExecutionError> = Ok(());

    assert!(shell_input_result_requires_session_drop(&channel_failure));
    assert!(!shell_input_result_requires_session_drop(&sftp_failure));
    assert!(!shell_input_result_requires_session_drop(&success));
}

#[test]
fn remote_shell_cache_drop_follows_shell_terminal_events() {
    let shell_session_id = session_id();
    let other_session_id = session_id();

    assert!(!remote_shell_events_require_cache_drop(
        shell_session_id,
        &[
            BackendEvent::Output {
                session_id: shell_session_id,
                line: "still running".to_owned(),
            },
            BackendEvent::SftpFailed {
                session_id: shell_session_id,
                reason: "unrelated sftp failure".to_owned(),
            },
        ],
    ));
    assert!(!remote_shell_events_require_cache_drop(
        shell_session_id,
        &[BackendEvent::Disconnected {
            session_id: other_session_id,
        }],
    ));
    assert!(remote_shell_events_require_cache_drop(
        shell_session_id,
        &[BackendEvent::CommandExited {
            session_id: shell_session_id,
            exit_code: Some(0),
        }],
    ));
    assert!(remote_shell_events_require_cache_drop(
        shell_session_id,
        &[BackendEvent::Failed {
            session_id: shell_session_id,
            reason: "channel failed".to_owned(),
        }],
    ));
    assert!(remote_shell_events_require_cache_drop(
        shell_session_id,
        &[BackendEvent::Disconnected {
            session_id: shell_session_id,
        }],
    ));
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
fn sftp_failure_drops_only_failed_cached_session() {
    let failed_session_id = session_id();
    let other_session_id = session_id();
    let result: Result<Vec<BackendEvent>, BackendExecutionError> =
        Err(sftp_error("list dir", "permission denied"));
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
        Err(channel_reason_error("read", "channel closed"));
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
fn sftp_drop_gate_is_strict_about_sftp_failures_only() {
    let sftp_failure: Result<Vec<BackendEvent>, BackendExecutionError> =
        Err(sftp_error("list dir", "permission denied"));
    let channel_failure: Result<Vec<BackendEvent>, BackendExecutionError> =
        Err(channel_reason_error("read", "channel closed"));
    let success: Result<Vec<BackendEvent>, BackendExecutionError> = Ok(Vec::new());

    assert!(sftp_result_requires_session_drop(&sftp_failure));
    assert!(!sftp_result_requires_session_drop(&channel_failure));
    assert!(!sftp_result_requires_session_drop(&success));
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
