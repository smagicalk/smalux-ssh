//! 会话启动和后端命令调度。

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use uuid::Uuid;

use crate::backend::{
    BackendCommand, ConnectionTarget, PtyRequest, RemoteCommandRequest, SftpRequest,
    TunnelStartRequest, TunnelStopRequest,
};
use crate::model::{
    CommandHistoryId, CommandHistoryItem, Host, HostId, RecentConnection, SessionId, SessionKind,
    SessionStatus, TransferDirection, TransferId, TransferStatus, TransferTask, TunnelRule,
};
use crate::terminal::TerminalTabState;

use super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 打开交互式 Shell，并把连接和 PTY 请求排入后端队列。
    pub(super) fn open_shell(&mut self, host_id: HostId) -> AppUpdateOutcome {
        let Some(host) = self.host_by_id(host_id) else {
            return missing_host(host_id);
        };

        let session_id = SessionId(Uuid::new_v4());
        let terminal_tab = TerminalTabState::new(session_id, host.name.clone());
        let pty = PtyRequest::xterm(terminal_tab.size);

        self.sessions
            .open_shell_tab(session_id, host.id, host.name.clone());
        self.sessions
            .set_status(session_id, SessionStatus::Connecting);
        self.terminal.open_tab(terminal_tab);
        self.record_recent_connection(&host);
        self.backend_commands.extend([
            connect_command(session_id, &host),
            BackendCommand::OpenShell { session_id, pty },
        ]);

        queued_outcome(2)
    }

    /// 从最近连接记录重新打开交互式 Shell。
    pub(super) fn open_recent_connection(&mut self, host_id: HostId) -> AppUpdateOutcome {
        self.open_shell(host_id)
    }

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

    /// 将本地文件上传到当前 SFTP 目录。
    pub(super) fn upload_sftp(&mut self, host_id: HostId) -> AppUpdateOutcome {
        let Some(session_id) = self.sftp_session_id_for_host(host_id) else {
            return missing_sftp_browser(host_id);
        };
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
        let remote_path = join_remote_path(&current_dir, &remote_name);
        let transfer_id = TransferId(Uuid::new_v4());

        self.sessions.enqueue_transfer(TransferTask {
            id: transfer_id,
            host_id,
            direction: TransferDirection::Upload,
            local_path: local_path.clone(),
            remote_path: remote_path.clone(),
            total_bytes: None,
            transferred_bytes: 0,
            status: TransferStatus::Queued,
        });
        self.sessions.set_sftp_loading(host_id, true);
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
        let Some(session_id) = self.sftp_session_id_for_host(host_id) else {
            return missing_sftp_browser(host_id);
        };

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
        let transfer_id = TransferId(Uuid::new_v4());

        self.sessions.enqueue_transfer(TransferTask {
            id: transfer_id,
            host_id,
            direction: TransferDirection::Download,
            local_path: local_path.clone(),
            remote_path: remote_path.clone(),
            total_bytes: None,
            transferred_bytes: 0,
            status: TransferStatus::Queued,
        });
        self.sessions.set_sftp_loading(host_id, true);
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

    /// 删除远程文件。
    pub(super) fn remove_sftp_file(
        &mut self,
        host_id: HostId,
        remote_path: String,
    ) -> AppUpdateOutcome {
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

        let remote_path = join_remote_path(&current_dir, &new_dir_name);
        self.queue_sftp_path_action(host_id, SftpRequest::CreateDir { remote_path })
    }

    /// 执行一次性远程命令，并记录主机作用域命令历史。
    pub(super) fn run_remote_command(
        &mut self,
        host_id: HostId,
        command: String,
        request_pty: bool,
    ) -> AppUpdateOutcome {
        let command = command.trim().to_owned();
        if command.is_empty() {
            return AppUpdateOutcome {
                error: Some("远程命令不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let Some(host) = self.host_by_id(host_id) else {
            return missing_host(host_id);
        };

        let session_id = SessionId(Uuid::new_v4());
        let terminal_tab = TerminalTabState::new(session_id, command.clone());
        let request = remote_command_request(command.clone(), terminal_tab.size, request_pty);

        self.sessions
            .open_remote_command_tab(session_id, host.id, command.clone());
        self.sessions
            .set_status(session_id, SessionStatus::Connecting);
        self.terminal.open_tab(terminal_tab);
        self.record_recent_connection(&host);
        self.record_command_history(host.id, command);
        self.backend_commands.extend([
            connect_command(session_id, &host),
            BackendCommand::RunCommand {
                session_id,
                request,
            },
        ]);

        queued_outcome(2)
    }

    /// 启动端口转发或动态隧道，并建立对应的管理标签页。
    pub(super) fn start_tunnel(&mut self, host_id: HostId, rule: TunnelRule) -> AppUpdateOutcome {
        let Some(host) = self.host_by_id(host_id) else {
            return missing_host(host_id);
        };
        let request = match TunnelStartRequest::new(rule.clone()) {
            Ok(request) => request,
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(format!("隧道规则无效：{error:?}")),
                    ..AppUpdateOutcome::default()
                };
            }
        };

        let session_id = SessionId(Uuid::new_v4());
        self.sessions.open_tunnel_tab(session_id, host.id, &rule);
        self.sessions
            .set_status(session_id, SessionStatus::Connecting);
        self.sessions
            .start_tunnel(&rule, Some(host.id), unix_now_secs());
        self.record_recent_connection(&host);
        self.backend_commands.extend([
            connect_command(session_id, &host),
            BackendCommand::StartTunnel {
                session_id,
                request,
            },
        ]);

        queued_outcome(2)
    }

    /// 停止指定隧道规则。
    pub(super) fn stop_tunnel(
        &mut self,
        session_id: SessionId,
        rule_name: String,
    ) -> AppUpdateOutcome {
        self.backend_commands.push(BackendCommand::StopTunnel {
            session_id,
            request: TunnelStopRequest::by_name(rule_name),
        });

        queued_outcome(1)
    }

    fn queue_sftp_list_dir(&mut self, host_id: HostId, remote_path: String) -> AppUpdateOutcome {
        self.queue_sftp_path_action(host_id, SftpRequest::ListDir { remote_path })
    }

    fn queue_sftp_path_action(
        &mut self,
        host_id: HostId,
        request: SftpRequest,
    ) -> AppUpdateOutcome {
        let Some(session_id) = self.sftp_session_id_for_host(host_id) else {
            return missing_sftp_browser(host_id);
        };

        self.sessions.set_sftp_loading(host_id, true);
        self.backend_commands.push(BackendCommand::Sftp {
            session_id,
            request,
        });

        queued_outcome(1)
    }

    fn host_by_id(&self, host_id: HostId) -> Option<Host> {
        self.storage
            .hosts
            .iter()
            .find(|host| host.id == host_id)
            .cloned()
    }

    fn current_sftp_dir_for_host(&self, host_id: HostId) -> Option<String> {
        self.sessions
            .sftp_browsers
            .iter()
            .find(|browser| browser.host_id == host_id)
            .map(|browser| browser.current_dir.clone())
    }

    fn sftp_session_id_for_host(&self, host_id: HostId) -> Option<SessionId> {
        self.sessions
            .tabs
            .iter()
            .rev()
            .find(|tab| tab.host_id == Some(host_id) && matches!(tab.kind, SessionKind::Sftp))
            .map(|tab| tab.id)
    }

    fn record_recent_connection(&mut self, host: &Host) {
        self.storage.record_recent_connection(RecentConnection {
            host_id: host.id,
            label: host.name.clone(),
            connected_at_unix_secs: unix_now_secs(),
        });
    }

    pub(super) fn record_command_history(&mut self, host_id: HostId, command: String) {
        self.storage.add_command_history(CommandHistoryItem {
            id: CommandHistoryId(Uuid::new_v4()),
            host_id: Some(host_id),
            command,
            working_directory: None,
            exit_code: None,
            started_at_unix_secs: unix_now_secs(),
            duration_ms: None,
        });
    }
}

fn connect_command(session_id: SessionId, host: &Host) -> BackendCommand {
    BackendCommand::Connect {
        session_id,
        target: ConnectionTarget::from_host(host),
    }
}

fn remote_command_request(
    command: String,
    size: crate::terminal::TerminalSize,
    request_pty: bool,
) -> RemoteCommandRequest {
    if request_pty {
        RemoteCommandRequest::with_pty(command, PtyRequest::xterm(size))
    } else {
        RemoteCommandRequest::exec(command)
    }
}

fn queued_outcome(queued_backend_commands: usize) -> AppUpdateOutcome {
    AppUpdateOutcome {
        state_changed: true,
        queued_backend_commands,
        ..AppUpdateOutcome::default()
    }
}

fn missing_host(host_id: HostId) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!("找不到主机：{}", host_id.0)),
        ..AppUpdateOutcome::default()
    }
}

fn normalize_remote_dir(remote_dir: &str) -> String {
    let remote_dir = remote_dir.trim();

    if remote_dir.is_empty() {
        "/".to_owned()
    } else {
        remote_dir.to_owned()
    }
}

fn join_remote_path(remote_dir: &str, name: &str) -> String {
    if remote_dir == "/" {
        format!("/{name}")
    } else {
        format!(
            "{}/{}",
            remote_dir.trim_end_matches('/'),
            name.trim_start_matches('/')
        )
    }
}

fn basename_local_path(path: &str) -> Option<String> {
    Path::new(path)
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .map(ToOwned::to_owned)
}

fn missing_sftp_browser(host_id: HostId) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!("找不到该主机的 SFTP 浏览器：{}", host_id.0)),
        ..AppUpdateOutcome::default()
    }
}

fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
