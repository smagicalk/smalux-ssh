//! SSH executor 失败后缓存清理判断。

use std::collections::HashMap;

use smagical_ssh_client_core::{is_channel_failure, is_sftp_failure};

use crate::backend::{BackendEvent, BackendExecutionError};
use crate::model::SessionId;

pub(in crate::backend::ssh::executor) fn drop_cached_shell_after_failed_input<T>(
    shells: &mut HashMap<SessionId, T>,
    session_id: SessionId,
    result: &Result<(), BackendExecutionError>,
) -> bool {
    if !shell_input_result_requires_session_drop(result) {
        return false;
    }

    shells.remove(&session_id).is_some()
}

pub(in crate::backend::ssh::executor) fn shell_input_result_requires_session_drop(
    result: &Result<(), BackendExecutionError>,
) -> bool {
    result.as_ref().is_err_and(is_channel_failure)
}

pub(in crate::backend::ssh::executor) fn drop_cached_sftp_after_failed_request<T>(
    sftps: &mut HashMap<SessionId, T>,
    session_id: SessionId,
    result: &Result<Vec<BackendEvent>, BackendExecutionError>,
) -> bool {
    if !sftp_result_requires_session_drop(result) {
        return false;
    }

    sftps.remove(&session_id).is_some()
}

pub(in crate::backend::ssh::executor) fn sftp_result_requires_session_drop(
    result: &Result<Vec<BackendEvent>, BackendExecutionError>,
) -> bool {
    result.as_ref().is_err_and(is_sftp_failure)
}

pub(in crate::backend::ssh::executor) fn remote_shell_events_require_cache_drop(
    session_id: SessionId,
    events: &[BackendEvent],
) -> bool {
    events
        .iter()
        .any(|event| remote_shell_event_requires_cache_drop(session_id, event))
}

fn remote_shell_event_requires_cache_drop(session_id: SessionId, event: &BackendEvent) -> bool {
    if event.session_id() != session_id {
        return false;
    }

    event.is_terminal()
}

#[cfg(test)]
mod tests {
    use super::*;
    use smagical_ssh_client_core::{channel_reason_error, sftp_error};
    use uuid::Uuid;

    fn session_id() -> SessionId {
        SessionId(Uuid::new_v4())
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
}
