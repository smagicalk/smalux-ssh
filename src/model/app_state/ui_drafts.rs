//! UI 输入草稿消息处理。

use crate::backend::BackendCommand;
use crate::model::HostId;
use crate::model::QuickHostAuthField;
use crate::model::QuickHostAuthKind;
use crate::model::QuickHostDraftField;
use crate::model::SessionId;
use crate::model::SessionKind;
use crate::model::SftpActionDraftField;
use crate::terminal::TerminalTabState;
use uuid::Uuid;

use super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 更新快速新增主机表单草稿。
    pub(super) fn update_quick_host_draft(
        &mut self,
        field: QuickHostDraftField,
        value: String,
    ) -> AppUpdateOutcome {
        self.ui.set_quick_host_field(field, value);
        draft_changed()
    }

    /// 更新快速新增主机认证方式。
    pub(super) fn update_quick_host_auth_kind(
        &mut self,
        kind: QuickHostAuthKind,
    ) -> AppUpdateOutcome {
        self.ui.set_quick_host_auth_kind(kind);
        draft_changed()
    }

    /// 更新快速新增主机认证字段。
    pub(super) fn update_quick_host_auth_field(
        &mut self,
        field: QuickHostAuthField,
        value: String,
    ) -> AppUpdateOutcome {
        self.ui.set_quick_host_auth_field(field, value);
        draft_changed()
    }

    /// 保存快速新增主机。
    pub(super) fn save_quick_host(&mut self) -> AppUpdateOutcome {
        let host_id = HostId(Uuid::new_v4());
        let host = match self.ui.quick_host.build_host(host_id) {
            Ok(host) => host,
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(format!("主机表单无效：{error}")),
                    ..AppUpdateOutcome::default()
                };
            }
        };

        self.storage.upsert_host(host);
        self.ui.reset_quick_host();
        draft_changed()
    }

    /// 更新某台主机的远程命令输入草稿。
    pub(super) fn update_host_command_draft(
        &mut self,
        host_id: HostId,
        command: String,
    ) -> AppUpdateOutcome {
        self.ui.set_remote_command(host_id, command);
        draft_changed()
    }

    /// 更新某台主机的 SFTP 初始路径输入草稿。
    pub(super) fn update_host_sftp_initial_dir_draft(
        &mut self,
        host_id: HostId,
        initial_dir: String,
    ) -> AppUpdateOutcome {
        self.ui.set_sftp_initial_dir(host_id, initial_dir);
        draft_changed()
    }

    /// 更新 SFTP 操作草稿。
    pub(super) fn update_sftp_action_draft(
        &mut self,
        host_id: HostId,
        field: SftpActionDraftField,
        value: String,
    ) -> AppUpdateOutcome {
        self.ui.set_sftp_action_field(host_id, field, value);
        draft_changed()
    }

    /// 更新终端输入草稿。
    pub(super) fn update_terminal_input_draft(
        &mut self,
        session_id: SessionId,
        input: String,
    ) -> AppUpdateOutcome {
        self.ui.set_terminal_input(session_id, input);
        draft_changed()
    }

    /// 向终端输入草稿追加可见字符。
    pub(super) fn append_terminal_input_draft(
        &mut self,
        session_id: SessionId,
        text: String,
    ) -> AppUpdateOutcome {
        let filtered = printable_terminal_input(&text);
        if filtered.is_empty() {
            return AppUpdateOutcome::default();
        }

        self.ui.append_terminal_input(session_id, filtered);
        draft_changed()
    }

    /// 删除终端输入草稿的最后一个字符。
    pub(super) fn backspace_terminal_input_draft(
        &mut self,
        session_id: SessionId,
    ) -> AppUpdateOutcome {
        let before = self.ui.terminal_input_for(session_id).to_owned();
        self.ui.backspace_terminal_input(session_id);
        AppUpdateOutcome {
            state_changed: before != self.ui.terminal_input_for(session_id),
            ..AppUpdateOutcome::default()
        }
    }

    /// 把当前终端输入草稿发送到 Shell 后端。
    pub(super) fn send_terminal_input(&mut self, session_id: SessionId) -> AppUpdateOutcome {
        if session_id == crate::model::LOCAL_TERMINAL_SESSION_ID {
            ensure_local_terminal_tab(self, session_id);
        }

        let Some(tab) = self
            .sessions
            .tabs
            .iter()
            .find(|tab| tab.id == session_id)
            .cloned()
        else {
            return AppUpdateOutcome {
                error: Some(format!("找不到会话：{}", session_id.0)),
                ..AppUpdateOutcome::default()
            };
        };

        if !matches!(tab.kind, SessionKind::LocalShell | SessionKind::Shell) {
            return AppUpdateOutcome {
                error: Some("只有 Shell 标签页支持交互输入".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }
        if !tab.can_accept_terminal_input() {
            return AppUpdateOutcome {
                error: Some("当前 Shell 会话不可交互，请重新连接后再发送输入".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let host_id = if matches!(tab.kind, SessionKind::LocalShell) {
            None
        } else {
            tab.host_id
        };
        if !matches!(tab.kind, SessionKind::LocalShell) && host_id.is_none() {
            return AppUpdateOutcome {
                error: Some("Shell 会话缺少主机标识".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        let input = self.ui.terminal_input_for(session_id).to_owned();
        let trimmed = input.trim().to_owned();
        if trimmed.is_empty() && !matches!(tab.kind, SessionKind::LocalShell) {
            return AppUpdateOutcome {
                error: Some("终端输入不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        if !trimmed.is_empty() {
            self.storage
                .add_command_history(crate::model::CommandHistoryItem {
                    id: crate::model::CommandHistoryId(Uuid::new_v4()),
                    host_id,
                    command: input.clone(),
                    working_directory: None,
                    exit_code: None,
                    started_at_unix_secs: unix_now_secs(),
                    duration_ms: None,
                });
        }

        self.backend_commands.push(BackendCommand::SendShellInput {
            session_id,
            input: format!("{input}\n"),
        });
        if matches!(tab.kind, SessionKind::LocalShell) && !trimmed.is_empty() {
            self.terminal.append_local_echo(
                session_id,
                crate::backend::LocalShellProfile::default_for_platform().prompt,
                &input,
            );
        }
        self.ui.clear_terminal_input(session_id);

        AppUpdateOutcome {
            state_changed: true,
            queued_backend_commands: 1,
            ..AppUpdateOutcome::default()
        }
    }
}

pub(super) fn draft_changed() -> AppUpdateOutcome {
    AppUpdateOutcome {
        state_changed: true,
        ..AppUpdateOutcome::default()
    }
}

fn printable_terminal_input(text: &str) -> String {
    text.chars()
        .filter(|ch| !ch.is_control() && !('\u{e000}'..='\u{f8ff}').contains(ch))
        .collect()
}

pub(super) fn ensure_local_terminal_tab(state: &mut AppState, session_id: SessionId) -> bool {
    let had_session = state.sessions.tabs.iter().any(|tab| tab.id == session_id);
    if !had_session {
        state
            .sessions
            .open_local_shell_tab(session_id, crate::model::DEFAULT_LOCAL_TERMINAL_TITLE);
    }

    let had_terminal = state
        .terminal
        .tabs
        .iter()
        .any(|tab| tab.session_id == session_id);
    if !had_terminal {
        state.terminal.open_tab(TerminalTabState::new(
            session_id,
            crate::model::DEFAULT_LOCAL_TERMINAL_TITLE,
        ));
    }

    !had_session || !had_terminal
}

fn unix_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
