use std::collections::HashMap;

use smagical_ssh_client_core::{channel_reason_error, sftp_error};

use crate::backend::{BackendEvent, BackendExecutionError};

use super::super::{drop_cached_sftp_after_failed_request, sftp_result_requires_session_drop};
use super::common::session_id;

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
