//! SFTP 上传、下载和远端路径操作调度。

use std::path::Path;

use uuid::Uuid;

use crate::backend::{BackendCommand, SftpRequest};
use crate::model::{
    HostId, SessionId, TransferDirection, TransferId, TransferStatus, TransferTask,
};

use super::launch::{join_remote_path, queued_outcome};
use super::launch_sftp::{missing_active_sftp_session, missing_sftp_browser};
use super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 将本地文件上传到当前 SFTP 目录。
    pub(super) fn upload_sftp(&mut self, host_id: HostId) -> AppUpdateOutcome {
        let Some(current_dir) = self.current_sftp_dir_for_host(host_id) else {
            return missing_sftp_browser(host_id);
        };

        let local_path = self.ui.sftp_local_path_for(host_id).trim().to_owned();
        if local_path.is_empty() {
            return AppUpdateOutcome {
                error: Some("SFTP 本地路径不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let remote_name = self.ui.sftp_remote_name_for(host_id).trim();
        let remote_name = if remote_name.is_empty() {
            match basename_local_path(&local_path) {
                Some(name) => name,
                None => {
                    return AppUpdateOutcome {
                        error: Some("无法从本地路径推断远程文件名".to_owned()),
                        ..AppUpdateOutcome::default()
                    };
                }
            }
        } else {
            remote_name.to_owned()
        };
        if !is_plain_remote_name(&remote_name) {
            return AppUpdateOutcome {
                error: Some("SFTP 远程文件名不能包含路径分隔符".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }
        let Some(session_id) = self.claim_sftp_session_id_for_host(host_id) else {
            return missing_active_sftp_session(host_id);
        };
        let remote_path = join_remote_path(&current_dir, &remote_name);
        let transfer_id = TransferId(Uuid::new_v4());

        self.sessions.enqueue_transfer(TransferTask {
            id: transfer_id,
            session_id,
            host_id,
            direction: TransferDirection::Upload,
            local_path: local_path.clone(),
            remote_path: remote_path.clone(),
            total_bytes: None,
            transferred_bytes: 0,
            status: TransferStatus::Queued,
        });
        self.sessions.set_sftp_loading_for_session(session_id, true);
        self.backend_commands.push(BackendCommand::Sftp {
            session_id,
            request: SftpRequest::Upload {
                id: transfer_id,
                local_path,
                remote_path,
            },
        });

        queued_outcome(1)
    }

    /// 将当前远程文件下载到本地路径草稿。
    pub(super) fn download_sftp(
        &mut self,
        host_id: HostId,
        remote_path: String,
    ) -> AppUpdateOutcome {
        let remote_path = remote_path.trim().to_owned();
        if remote_path.is_empty() || remote_path == "/" {
            return AppUpdateOutcome {
                error: Some("SFTP 下载路径不能为空或根目录".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let local_path = self.ui.sftp_local_path_for(host_id).trim().to_owned();
        let local_path = if local_path.is_empty() {
            match basename_local_path(&remote_path) {
                Some(name) => name,
                None => {
                    return AppUpdateOutcome {
                        error: Some("SFTP 本地路径不能为空".to_owned()),
                        ..AppUpdateOutcome::default()
                    };
                }
            }
        } else {
            local_path
        };
        let Some(session_id) = self.claim_sftp_session_id_for_host(host_id) else {
            return missing_active_sftp_session(host_id);
        };
        let transfer_id = TransferId(Uuid::new_v4());

        self.sessions.enqueue_transfer(TransferTask {
            id: transfer_id,
            session_id,
            host_id,
            direction: TransferDirection::Download,
            local_path: local_path.clone(),
            remote_path: remote_path.clone(),
            total_bytes: None,
            transferred_bytes: 0,
            status: TransferStatus::Queued,
        });
        self.backend_commands.push(BackendCommand::Sftp {
            session_id,
            request: SftpRequest::Download {
                id: transfer_id,
                remote_path,
                local_path,
            },
        });

        queued_outcome(1)
    }

    /// 取消尚未交给后端执行器的 SFTP 传输。
    pub(super) fn cancel_sftp_transfer(&mut self, transfer_id: TransferId) -> AppUpdateOutcome {
        let task = match unique_transfer_task(&self.sessions.transfers, transfer_id) {
            TransferLookup::Found(task) => task,
            TransferLookup::Missing => {
                return AppUpdateOutcome {
                    error: Some(format!("找不到 SFTP 传输任务：{}", transfer_id.0)),
                    ..AppUpdateOutcome::default()
                };
            }
            TransferLookup::Ambiguous => {
                return AppUpdateOutcome {
                    error: Some(format!("SFTP 传输任务不唯一，无法取消：{}", transfer_id.0)),
                    ..AppUpdateOutcome::default()
                };
            }
        };

        if !task.status.is_queued() {
            return AppUpdateOutcome {
                error: Some("只能取消尚未开始的 SFTP 传输".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let removed_commands = self
            .backend_commands
            .retain(|command| !is_sftp_transfer_command(command, task.session_id, transfer_id));
        if removed_commands == 0 {
            return AppUpdateOutcome {
                error: Some("SFTP 传输已经开始，无法从队列取消".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let has_pending_browser_refresh = has_pending_sftp_browser_refresh(
            &self.sessions,
            &self.backend_commands,
            task.host_id,
            task.session_id,
        );
        let transfer_cancelled = self
            .sessions
            .cancel_queued_transfer(task.session_id, transfer_id);
        let loading_cleared = clear_loading_for_cancelled_transfer(
            &mut self.sessions,
            &task,
            has_pending_browser_refresh,
        );

        AppUpdateOutcome {
            state_changed: transfer_cancelled || loading_cleared || removed_commands > 0,
            ..AppUpdateOutcome::default()
        }
    }

    /// 删除远程文件。
    pub(super) fn remove_sftp_file(
        &mut self,
        host_id: HostId,
        remote_path: String,
    ) -> AppUpdateOutcome {
        let remote_path = remote_path.trim().to_owned();
        if remote_path.is_empty() || remote_path == "/" {
            return AppUpdateOutcome {
                error: Some("SFTP 删除路径不能为空或根目录".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.queue_sftp_path_action(host_id, SftpRequest::RemoveFile { remote_path })
    }

    /// 在当前远程目录创建子目录。
    pub(super) fn create_sftp_dir(&mut self, host_id: HostId) -> AppUpdateOutcome {
        let Some(current_dir) = self.current_sftp_dir_for_host(host_id) else {
            return missing_sftp_browser(host_id);
        };

        let new_dir_name = self.ui.sftp_new_dir_name_for(host_id).trim().to_owned();
        if new_dir_name.is_empty() {
            return AppUpdateOutcome {
                error: Some("SFTP 新目录名不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }
        if !is_plain_remote_name(&new_dir_name) {
            return AppUpdateOutcome {
                error: Some("SFTP 新目录名不能包含路径分隔符".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let remote_path = join_remote_path(&current_dir, &new_dir_name);
        self.queue_sftp_path_action(host_id, SftpRequest::CreateDir { remote_path })
    }
}

fn basename_local_path(path: &str) -> Option<String> {
    Path::new(path)
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .map(ToOwned::to_owned)
}

fn is_plain_remote_name(name: &str) -> bool {
    !matches!(name, "." | "..") && !name.contains('/') && !name.contains('\\')
}

fn unique_transfer_task(tasks: &[TransferTask], transfer_id: TransferId) -> TransferLookup {
    let mut matches = tasks.iter().filter(|task| task.id == transfer_id);
    let Some(task) = matches.next() else {
        return TransferLookup::Missing;
    };
    if matches.next().is_some() {
        return TransferLookup::Ambiguous;
    }

    TransferLookup::Found(task.clone())
}

enum TransferLookup {
    Found(TransferTask),
    Missing,
    Ambiguous,
}

fn is_sftp_transfer_command(
    command: &BackendCommand,
    task_session_id: SessionId,
    transfer_id: TransferId,
) -> bool {
    matches!(
        command,
        BackendCommand::Sftp {
            session_id,
            request:
                SftpRequest::Upload { id, .. } | SftpRequest::Download { id, .. },
            ..
        } if *session_id == task_session_id && *id == transfer_id
    )
}

fn clear_loading_for_cancelled_transfer(
    sessions: &mut crate::session::SessionManager,
    task: &TransferTask,
    has_pending_browser_refresh: bool,
) -> bool {
    if matches!(task.direction, TransferDirection::Upload) && !has_pending_browser_refresh {
        sessions.set_sftp_loading_for_session(task.session_id, false)
    } else {
        false
    }
}

fn has_pending_sftp_browser_refresh(
    sessions: &crate::session::SessionManager,
    commands: &crate::backend::BackendCommandQueue,
    host_id: HostId,
    current_session_id: SessionId,
) -> bool {
    commands.iter().any(|command| {
        let BackendCommand::Sftp {
            session_id,
            request,
        } = command
        else {
            return false;
        };

        request.refreshes_browser()
            && *session_id == current_session_id
            && session_matches_host(sessions, *session_id, host_id)
    })
}

fn session_matches_host(
    sessions: &crate::session::SessionManager,
    session_id: SessionId,
    host_id: HostId,
) -> bool {
    sessions
        .tabs
        .iter()
        .any(|tab| tab.id == session_id && tab.host_id == Some(host_id))
}
