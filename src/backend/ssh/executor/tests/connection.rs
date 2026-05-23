use super::super::RusshBackendExecutor;
use super::common::{session_id, target};
use crate::backend::{BackendAuth, BackendCommand, BackendEvent, BackendExecutionError};
use crate::model::{HostKeyVerification, KeyAlgorithm, SecretRef};
use crate::security::MemorySecretStore;
use smagical_backend_core::BackendExecutor;

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
fn disconnect_without_connection_still_emits_disconnected_event() {
    let mut executor =
        RusshBackendExecutor::new(MemorySecretStore::new()).expect("执行器应该可以创建 runtime");
    let session_id = session_id();

    let events = executor
        .execute(BackendCommand::Disconnect { session_id })
        .expect("断开缺失连接应该保持幂等");

    assert_eq!(events, vec![BackendEvent::Disconnected { session_id }]);
}
