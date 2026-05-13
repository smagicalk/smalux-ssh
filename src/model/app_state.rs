//! Iced 应用根状态和消息调度。

use iced::Task;
use iced::Theme;

use std::fmt;

use crate::backend::{BackendCommand, BackendCommandQueue, BackendEvent, apply_backend_event};
use crate::backend::{
    SharedBackendExecutor, noop_shared_backend_executor, shared_backend_executor,
};
use crate::config::AppConfig;
use crate::session::SessionManager;
use crate::storage::{RedbStorage, StorageManager, StoragePersistenceError};
use crate::terminal::TerminalManager;

use super::{
    HostId, QuickHostAuthField, QuickHostAuthKind, QuickHostDraftField, SessionId, SessionKind,
    SessionStatus, SessionTab, SftpActionDraftField, TunnelRule, TunnelStatus, UiState,
    VisualSettingsDraftField,
};

mod backend_pump;
#[cfg(test)]
mod backend_pump_tests;
mod launch;
#[cfg(test)]
mod launch_tests;
mod storage_admin;
#[cfg(test)]
mod tests;
mod ui_drafts;
mod visual_settings;
mod workspace;

/// Iced 应用的根状态。
///
/// 根状态只组合各个单一职责管理器，不直接实现 SSH、SFTP 或终端细节。
#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub sessions: SessionManager,
    pub storage: StorageManager,
    pub storage_backend: Option<RedbStorage>,
    pub terminal: TerminalManager,
    pub ui: UiState,
    pub backend_commands: BackendCommandQueue,
    pub backend_executor: SharedBackendExecutor,
    pub theme: Theme,
}

impl fmt::Debug for AppState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppState")
            .field("config", &self.config)
            .field("sessions", &self.sessions)
            .field("storage", &self.storage)
            .field("storage_backend", &self.storage_backend)
            .field("terminal", &self.terminal)
            .field("ui", &self.ui)
            .field("backend_commands", &self.backend_commands)
            .field("backend_executor", &"<shared backend executor>")
            .field("theme", &self.theme)
            .finish()
    }
}

impl Default for AppState {
    fn default() -> Self {
        let config = AppConfig::default();
        let mut storage = StorageManager::default();
        storage.app_config = config.clone();
        let ui = UiState::from_visual(&config.theme, &config.background);

        Self {
            config,
            sessions: SessionManager::default(),
            storage,
            storage_backend: None,
            terminal: TerminalManager::default(),
            ui,
            backend_commands: BackendCommandQueue::default(),
            backend_executor: noop_shared_backend_executor(),
            theme: Theme::Dark,
        }
    }
}

/// UI 与后台任务之间传递的消息。
#[derive(Debug, Clone)]
pub enum Message {
    ToggleTheme,
    UpdateVisualSettingsDraft {
        field: VisualSettingsDraftField,
        value: String,
    },
    SetVisualBackgroundEnabled {
        enabled: bool,
    },
    ApplyVisualSettings,
    UpdateHostVisualSettingsDraft {
        host_id: HostId,
        field: VisualSettingsDraftField,
        value: String,
    },
    SetHostVisualBackgroundEnabled {
        host_id: HostId,
        enabled: bool,
    },
    ApplyHostVisualSettings {
        host_id: HostId,
    },
    ClearHostVisualSettings {
        host_id: HostId,
    },
    SaveWorkspaceSnapshot,
    RestoreWorkspaceSnapshot,
    ClearWorkspaceSnapshot,
    UpdateQuickHostDraft {
        field: QuickHostDraftField,
        value: String,
    },
    UpdateQuickHostAuthKind {
        kind: QuickHostAuthKind,
    },
    UpdateQuickHostAuthField {
        field: QuickHostAuthField,
        value: String,
    },
    SaveQuickHost,
    RemoveCredential {
        name: String,
    },
    TrustKnownHost {
        host: String,
        port: u16,
    },
    RemoveKnownHost {
        host: String,
        port: u16,
    },
    DismissUiError,
    CloseSessionTab {
        session_id: SessionId,
    },
    ActivateTerminalTab {
        session_id: SessionId,
    },
    UpdateTerminalInputDraft {
        session_id: SessionId,
        input: String,
    },
    SendTerminalInput {
        session_id: SessionId,
    },
    UpdateHostCommandDraft {
        host_id: HostId,
        command: String,
    },
    UpdateHostSftpInitialDirDraft {
        host_id: HostId,
        initial_dir: String,
    },
    UpdateSftpActionDraft {
        host_id: HostId,
        field: SftpActionDraftField,
        value: String,
    },
    RefreshSftp {
        host_id: HostId,
    },
    NavigateSftp {
        host_id: HostId,
        remote_path: String,
    },
    SelectSftpEntry {
        host_id: HostId,
        remote_path: String,
    },
    UploadSftp {
        host_id: HostId,
    },
    DownloadSftp {
        host_id: HostId,
        remote_path: String,
    },
    RemoveSftpFile {
        host_id: HostId,
        remote_path: String,
    },
    CreateSftpDir {
        host_id: HostId,
    },
    OpenShell {
        host_id: HostId,
    },
    OpenRecentConnection {
        host_id: HostId,
    },
    OpenSftp {
        host_id: HostId,
        initial_dir: String,
    },
    RunRemoteCommand {
        host_id: HostId,
        command: String,
        request_pty: bool,
    },
    StartTunnel {
        host_id: HostId,
        rule: TunnelRule,
    },
    StopTunnel {
        session_id: SessionId,
        rule_name: String,
    },
    BackendEventReceived(BackendEvent),
}

/// 应用消息处理结果。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppUpdateOutcome {
    pub state_changed: bool,
    pub queued_backend_commands: usize,
    pub executed_backend_commands: usize,
    pub applied_backend_events: usize,
    pub error: Option<String>,
}

impl AppUpdateOutcome {
    /// 是否有状态变化或错误反馈。
    pub fn changed(&self) -> bool {
        self.state_changed || self.error.is_some()
    }
}

impl AppState {
    /// 使用指定共享执行器替换默认占位执行器。
    pub fn with_backend_executor<E>(mut self, executor: E) -> Self
    where
        E: crate::backend::BackendExecutor + 'static,
    {
        self.backend_executor = shared_backend_executor(executor);
        self
    }

    /// 使用指定本地存储后端启用持久化。
    pub fn with_storage_backend(mut self, storage_backend: RedbStorage) -> Self {
        self.storage_backend = Some(storage_backend);
        self
    }

    /// 从已配置的本地存储后端保存当前持久化状态。
    pub fn persist_storage(&self) -> Result<(), StoragePersistenceError> {
        if let Some(storage_backend) = &self.storage_backend {
            storage_backend.save(&self.storage)?;
        }

        Ok(())
    }

    /// 构造 Iced 启动需要的初始状态和首个任务。
    pub fn boot() -> (Self, Task<Message>) {
        (Self::default(), Task::none())
    }

    /// 将 UI 消息应用到根状态。
    pub fn apply(&mut self, message: Message) -> AppUpdateOutcome {
        let mut outcome = match message {
            Message::ToggleTheme => self.toggle_theme(),
            Message::UpdateVisualSettingsDraft { field, value } => {
                self.update_visual_settings_draft(field, value)
            }
            Message::SetVisualBackgroundEnabled { enabled } => {
                self.set_visual_background_enabled(enabled)
            }
            Message::ApplyVisualSettings => self.apply_visual_settings(),
            Message::UpdateHostVisualSettingsDraft {
                host_id,
                field,
                value,
            } => self.update_host_visual_settings_draft(host_id, field, value),
            Message::SetHostVisualBackgroundEnabled { host_id, enabled } => {
                self.set_host_visual_background_enabled(host_id, enabled)
            }
            Message::ApplyHostVisualSettings { host_id } => {
                self.apply_host_visual_settings(host_id)
            }
            Message::ClearHostVisualSettings { host_id } => {
                self.clear_host_visual_settings(host_id)
            }
            Message::SaveWorkspaceSnapshot => self.save_workspace_snapshot(),
            Message::RestoreWorkspaceSnapshot => self.restore_workspace_snapshot(),
            Message::ClearWorkspaceSnapshot => self.clear_workspace_snapshot(),
            Message::UpdateQuickHostDraft { field, value } => {
                self.update_quick_host_draft(field, value)
            }
            Message::UpdateQuickHostAuthKind { kind } => self.update_quick_host_auth_kind(kind),
            Message::UpdateQuickHostAuthField { field, value } => {
                self.update_quick_host_auth_field(field, value)
            }
            Message::SaveQuickHost => self.save_quick_host(),
            Message::RemoveCredential { name } => self.remove_credential(&name),
            Message::TrustKnownHost { host, port } => self.trust_known_host(&host, port),
            Message::RemoveKnownHost { host, port } => self.remove_known_host(&host, port),
            Message::DismissUiError => self.dismiss_ui_error(),
            Message::CloseSessionTab { session_id } => self.close_session_tab(session_id),
            Message::ActivateTerminalTab { session_id } => {
                if self.terminal.set_active_tab(session_id) {
                    self.sessions.active_tab = Some(session_id);
                    AppUpdateOutcome {
                        state_changed: true,
                        ..AppUpdateOutcome::default()
                    }
                } else {
                    AppUpdateOutcome {
                        error: Some(format!("找不到终端标签页：{}", session_id.0)),
                        ..AppUpdateOutcome::default()
                    }
                }
            }
            Message::UpdateTerminalInputDraft { session_id, input } => {
                self.update_terminal_input_draft(session_id, input)
            }
            Message::SendTerminalInput { session_id } => self.send_terminal_input(session_id),
            Message::UpdateHostCommandDraft { host_id, command } => {
                self.update_host_command_draft(host_id, command)
            }
            Message::UpdateHostSftpInitialDirDraft {
                host_id,
                initial_dir,
            } => self.update_host_sftp_initial_dir_draft(host_id, initial_dir),
            Message::UpdateSftpActionDraft {
                host_id,
                field,
                value,
            } => self.update_sftp_action_draft(host_id, field, value),
            Message::RefreshSftp { host_id } => self.refresh_sftp(host_id),
            Message::NavigateSftp {
                host_id,
                remote_path,
            } => self.navigate_sftp(host_id, remote_path),
            Message::SelectSftpEntry {
                host_id,
                remote_path,
            } => self.select_sftp_entry(host_id, remote_path),
            Message::UploadSftp { host_id } => self.upload_sftp(host_id),
            Message::DownloadSftp {
                host_id,
                remote_path,
            } => self.download_sftp(host_id, remote_path),
            Message::RemoveSftpFile {
                host_id,
                remote_path,
            } => self.remove_sftp_file(host_id, remote_path),
            Message::CreateSftpDir { host_id } => self.create_sftp_dir(host_id),
            Message::OpenShell { host_id } => self.open_shell(host_id),
            Message::OpenRecentConnection { host_id } => self.open_recent_connection(host_id),
            Message::OpenSftp {
                host_id,
                initial_dir,
            } => self.open_sftp(host_id, initial_dir),
            Message::RunRemoteCommand {
                host_id,
                command,
                request_pty,
            } => self.run_remote_command(host_id, command, request_pty),
            Message::StartTunnel { host_id, rule } => self.start_tunnel(host_id, rule),
            Message::StopTunnel {
                session_id,
                rule_name,
            } => self.stop_tunnel(session_id, rule_name),
            Message::BackendEventReceived(event) => self.apply_backend_event(event),
        };

        if let Some(error) = &outcome.error {
            outcome.state_changed |= self.ui.set_last_error(error.clone());
        }

        outcome
    }

    fn toggle_theme(&mut self) -> AppUpdateOutcome {
        self.theme = if matches!(self.theme, Theme::Dark) {
            Theme::Light
        } else {
            Theme::Dark
        };

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    fn dismiss_ui_error(&mut self) -> AppUpdateOutcome {
        AppUpdateOutcome {
            state_changed: self.ui.clear_last_error(),
            ..AppUpdateOutcome::default()
        }
    }

    fn close_session_tab(&mut self, session_id: SessionId) -> AppUpdateOutcome {
        let Some(tab) = self
            .sessions
            .tabs
            .iter()
            .find(|tab| tab.id == session_id)
            .cloned()
        else {
            return AppUpdateOutcome {
                error: Some(format!("找不到会话标签页：{}", session_id.0)),
                ..AppUpdateOutcome::default()
            };
        };

        if let SessionKind::Tunnel { rule_name } = &tab.kind {
            if self.tunnel_requires_stop_before_close(rule_name) {
                return AppUpdateOutcome {
                    error: Some(format!("隧道 {rule_name} 仍在运行，请先停止再关闭标签页")),
                    ..AppUpdateOutcome::default()
                };
            }
        }

        let should_disconnect = should_disconnect_on_close(&tab);
        let session_closed = self.sessions.close_tab(session_id);
        let terminal_closed = self.terminal.close_tab(session_id);
        let sftp_browser_removed = self.remove_sftp_browser_after_tab_close(&tab);
        let tunnel_runtime_removed = self.remove_tunnel_runtime_after_tab_close(&tab);

        if should_disconnect {
            self.backend_commands
                .push(BackendCommand::Disconnect { session_id });
        }

        AppUpdateOutcome {
            state_changed: session_closed
                || terminal_closed
                || sftp_browser_removed
                || tunnel_runtime_removed,
            queued_backend_commands: usize::from(should_disconnect),
            ..AppUpdateOutcome::default()
        }
    }

    fn tunnel_requires_stop_before_close(&self, rule_name: &str) -> bool {
        self.sessions.tunnels.iter().any(|tunnel| {
            tunnel.rule_name == rule_name
                && matches!(
                    tunnel.status,
                    TunnelStatus::Starting | TunnelStatus::Running | TunnelStatus::Stopping
                )
        })
    }

    fn remove_sftp_browser_after_tab_close(&mut self, tab: &SessionTab) -> bool {
        if !matches!(tab.kind, SessionKind::Sftp) {
            return false;
        }

        let Some(host_id) = tab.host_id else {
            return false;
        };
        let has_other_sftp_tab =
            self.sessions.tabs.iter().any(|other| {
                other.host_id == Some(host_id) && matches!(other.kind, SessionKind::Sftp)
            });

        if has_other_sftp_tab {
            return false;
        }

        let before = self.sessions.sftp_browsers.len();
        self.sessions
            .sftp_browsers
            .retain(|browser| browser.host_id != host_id);
        before != self.sessions.sftp_browsers.len()
    }

    fn remove_tunnel_runtime_after_tab_close(&mut self, tab: &SessionTab) -> bool {
        let SessionKind::Tunnel { rule_name } = &tab.kind else {
            return false;
        };

        let before = self.sessions.tunnels.len();
        self.sessions
            .tunnels
            .retain(|tunnel| tunnel.rule_name != *rule_name);
        before != self.sessions.tunnels.len()
    }

    fn apply_backend_event(&mut self, event: BackendEvent) -> AppUpdateOutcome {
        let outcome = apply_backend_event(&mut self.sessions, &mut self.terminal, event);

        AppUpdateOutcome {
            state_changed: outcome.changed(),
            applied_backend_events: 1,
            ..AppUpdateOutcome::default()
        }
    }

    /// 使用当前共享执行器泵出已排队的后台命令。
    pub fn drain_backend_queue_with_executor(&mut self) -> AppUpdateOutcome {
        let backend_executor = self.backend_executor.clone();
        let mut executor = backend_executor
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        self.drain_backend_queue(&mut **executor)
    }
}

fn should_disconnect_on_close(tab: &SessionTab) -> bool {
    !matches!(tab.kind, SessionKind::Tunnel { .. })
        && !matches!(
            tab.status,
            SessionStatus::Disconnected | SessionStatus::Failed { .. }
        )
}
