//! SFTP 会话启动、目录浏览和书签命令调度。

use uuid::Uuid;

use crate::backend::{BackendCommand, SftpRequest};
use crate::model::{HostId, SessionId, SessionKind, SessionStatus, SftpBookmark, WorkspacePage};

use super::launch::{connect_command, missing_host, normalize_remote_dir, queued_outcome};
use super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 打开 SFTP 浏览器，并排队读取初始远端目录。
    pub(super) fn open_sftp(&mut self, host_id: HostId, initial_dir: String) -> AppUpdateOutcome {
        let initial_dir = normalize_remote_dir(&initial_dir);
        let Some(host) = self.host_by_id(host_id) else {
            return missing_host(host_id);
        };

        let session_id = SessionId(Uuid::new_v4());
        self.sessions
            .open_sftp_tab(session_id, host.id, initial_dir.clone());
        self.sessions
            .set_status(session_id, SessionStatus::Connecting);
        self.sessions.set_sftp_loading(host.id, true);
        self.ui.workspace.active_page = WorkspacePage::Sftp;
        self.record_recent_connection(&host);
        self.backend_commands.extend([
            connect_command(session_id, &host),
            BackendCommand::Sftp {
                session_id,
                request: SftpRequest::ListDir {
                    remote_path: initial_dir,
                },
            },
        ]);

        queued_outcome(2)
    }

    /// 刷新当前 SFTP 目录。
    pub(super) fn refresh_sftp(&mut self, host_id: HostId) -> AppUpdateOutcome {
        let Some(current_dir) = self.current_sftp_dir_for_host(host_id) else {
            return missing_sftp_browser(host_id);
        };

        self.queue_sftp_list_dir(host_id, current_dir)
    }

    /// 将当前 SFTP 浏览目录保存为书签。
    pub(super) fn save_sftp_bookmark(&mut self, host_id: HostId) -> AppUpdateOutcome {
        let Some(current_dir) = self.current_sftp_dir_for_host(host_id) else {
            return missing_sftp_browser(host_id);
        };

        self.storage.upsert_sftp_bookmark(SftpBookmark {
            host_id,
            label: sftp_bookmark_label(&current_dir),
            remote_path: current_dir,
        });

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    /// 打开 SFTP 书签；已有浏览器时导航，否则新开 SFTP 标签页。
    pub(super) fn open_sftp_bookmark(
        &mut self,
        host_id: HostId,
        remote_path: String,
    ) -> AppUpdateOutcome {
        let remote_path = normalize_remote_dir(&remote_path);

        if self.sftp_session_id_for_host(host_id).is_some() {
            self.queue_sftp_list_dir(host_id, remote_path)
        } else {
            self.open_sftp(host_id, remote_path)
        }
    }

    /// 删除指定 SFTP 书签。
    pub(super) fn remove_sftp_bookmark(
        &mut self,
        host_id: HostId,
        remote_path: String,
    ) -> AppUpdateOutcome {
        if self.storage.remove_sftp_bookmark(host_id, &remote_path) {
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            AppUpdateOutcome {
                error: Some(format!("找不到 SFTP 书签：{remote_path}")),
                ..AppUpdateOutcome::default()
            }
        }
    }

    /// 切换到指定 SFTP 目录。
    pub(super) fn navigate_sftp(
        &mut self,
        host_id: HostId,
        remote_path: String,
    ) -> AppUpdateOutcome {
        let remote_path = normalize_remote_dir(&remote_path);

        self.queue_sftp_list_dir(host_id, remote_path)
    }

    /// 记录当前选中的 SFTP 目录项，不触发后端请求。
    pub(super) fn select_sftp_entry(
        &mut self,
        host_id: HostId,
        remote_path: String,
    ) -> AppUpdateOutcome {
        if self.sessions.select_sftp_entry(host_id, remote_path) {
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            missing_sftp_browser(host_id)
        }
    }

    fn queue_sftp_list_dir(&mut self, host_id: HostId, remote_path: String) -> AppUpdateOutcome {
        self.queue_sftp_path_action(host_id, SftpRequest::ListDir { remote_path })
    }

    pub(super) fn queue_sftp_path_action(
        &mut self,
        host_id: HostId,
        request: SftpRequest,
    ) -> AppUpdateOutcome {
        let Some(session_id) = self.claim_sftp_session_id_for_host(host_id) else {
            return missing_active_sftp_session(host_id);
        };

        self.sessions.set_sftp_loading(host_id, true);
        self.backend_commands.push(BackendCommand::Sftp {
            session_id,
            request,
        });

        queued_outcome(1)
    }

    pub(super) fn current_sftp_dir_for_host(&self, host_id: HostId) -> Option<String> {
        self.sessions
            .sftp_browsers
            .iter()
            .find(|browser| browser.host_id == host_id)
            .map(|browser| browser.current_dir.clone())
    }

    pub(super) fn sftp_session_id_for_host(&self, host_id: HostId) -> Option<SessionId> {
        self.sftp_browser_owner_session_id(host_id)
            .filter(|session_id| self.sftp_session_can_accept_commands(*session_id, host_id))
            .or_else(|| self.fallback_sftp_session_id_for_host(host_id))
    }

    pub(super) fn claim_sftp_session_id_for_host(&mut self, host_id: HostId) -> Option<SessionId> {
        let session_id = self.sftp_session_id_for_host(host_id)?;
        self.sessions
            .reassign_sftp_browser_session(host_id, session_id);
        Some(session_id)
    }

    fn sftp_browser_owner_session_id(&self, host_id: HostId) -> Option<SessionId> {
        self.sessions
            .sftp_browsers
            .iter()
            .find(|browser| browser.host_id == host_id)
            .map(|browser| browser.session_id)
    }

    fn fallback_sftp_session_id_for_host(&self, host_id: HostId) -> Option<SessionId> {
        self.sessions
            .tabs
            .iter()
            .rev()
            .find(|tab| {
                tab.host_id == Some(host_id)
                    && matches!(tab.kind, SessionKind::Sftp)
                    && sftp_tab_can_accept_commands(&tab.status)
            })
            .map(|tab| tab.id)
    }

    fn sftp_session_can_accept_commands(&self, session_id: SessionId, host_id: HostId) -> bool {
        self.sessions.tabs.iter().any(|tab| {
            tab.id == session_id
                && tab.host_id == Some(host_id)
                && matches!(tab.kind, SessionKind::Sftp)
                && sftp_tab_can_accept_commands(&tab.status)
        })
    }
}

fn sftp_bookmark_label(remote_path: &str) -> String {
    if remote_path == "/" {
        return "/".to_owned();
    }

    remote_path
        .trim_end_matches('/')
        .rsplit('/')
        .find(|part| !part.is_empty())
        .unwrap_or(remote_path)
        .to_owned()
}

pub(super) fn missing_sftp_browser(host_id: HostId) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!("找不到该主机的 SFTP 浏览器：{}", host_id.0)),
        ..AppUpdateOutcome::default()
    }
}

pub(super) fn missing_active_sftp_session(host_id: HostId) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!("该主机没有可用的 SFTP 会话：{}", host_id.0)),
        ..AppUpdateOutcome::default()
    }
}

fn sftp_tab_can_accept_commands(status: &SessionStatus) -> bool {
    !matches!(
        status,
        SessionStatus::Disconnected | SessionStatus::Failed { .. }
    )
}
