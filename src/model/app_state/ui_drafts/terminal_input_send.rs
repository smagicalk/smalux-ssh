//! 终端输入发送处理。

use crate::backend::BackendCommand;
use crate::model::{SessionId, SessionKind};

use super::super::{AppState, AppUpdateOutcome};
use super::local_terminal::ensure_local_terminal_tab;
use terminal_input_echo::echo_local_terminal_input;
use terminal_input_history::record_terminal_input_history;

#[path = "terminal_input_echo.rs"]
mod terminal_input_echo;
#[path = "terminal_input_history.rs"]
mod terminal_input_history;

impl AppState {
    /// 把当前终端输入草稿发送到 Shell 后端。
    pub(in crate::model::app_state) fn send_terminal_input(
        &mut self,
        session_id: SessionId,
    ) -> AppUpdateOutcome {
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
            record_terminal_input_history(&mut self.storage, host_id, input.clone());
        }

        self.backend_commands.push(BackendCommand::SendShellInput {
            session_id,
            input: format!("{input}\n"),
        });
        echo_local_terminal_input(&mut self.terminal, session_id, &tab.kind, &trimmed, &input);
        self.ui.clear_terminal_input(session_id);

        AppUpdateOutcome {
            state_changed: true,
            queued_backend_commands: 1,
            ..AppUpdateOutcome::default()
        }
    }
}
