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
#[path = "drop_gates_tests.rs"]
mod tests;
