use std::collections::HashMap;

use smagical_ssh_client_core::{channel_reason_error, sftp_error};

use crate::backend::{BackendEvent, BackendExecutionError};

use super::super::{
    drop_cached_shell_after_failed_input, remote_shell_events_require_cache_drop,
    shell_input_result_requires_session_drop,
};
use super::common::session_id;

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
