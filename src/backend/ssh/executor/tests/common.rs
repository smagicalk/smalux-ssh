use crate::backend::{BackendAuth, BackendExecutionError, ConnectionTarget};
use crate::model::{HostId, SessionId};
use smagical_ssh_client_core::channel_failure_parts;
use uuid::Uuid;

pub(super) fn session_id() -> SessionId {
    SessionId(Uuid::new_v4())
}

pub(super) fn target(auth: BackendAuth) -> ConnectionTarget {
    ConnectionTarget {
        host_id: HostId(Uuid::new_v4()),
        address: "example.com".to_owned(),
        port: 22,
        auth,
        known_hosts: Vec::new(),
    }
}

pub(super) fn assert_channel_failure(
    error: &BackendExecutionError,
    expected_operation: &str,
    expected_reason: &str,
) {
    let (operation, reason) = channel_failure_parts(error).expect("错误应该是 SSH channel 失败");
    assert_eq!(operation, expected_operation);
    assert_eq!(reason, expected_reason);
}
