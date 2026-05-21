//! 终端输入草稿和发送处理。

use uuid::Uuid;

use crate::backend::BackendCommand;
use crate::model::{SessionId, SessionKind};

use super::super::{AppState, AppUpdateOutcome};
use super::draft_changed;
use super::local_terminal::ensure_local_terminal_tab;

impl AppState {
    /// 更新终端输入草稿。
    pub(in crate::model::app_state) fn update_terminal_input_draft(
        &mut self,
        session_id: SessionId,
        input: String,
    ) -> AppUpdateOutcome {
        self.ui.set_terminal_input(session_id, input);
        draft_changed()
    }

    /// 向终端输入草稿追加可见字符。
    pub(in crate::model::app_state) fn append_terminal_input_draft(
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
    pub(in crate::model::app_state) fn backspace_terminal_input_draft(
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

fn printable_terminal_input(text: &str) -> String {
    text.chars()
        .filter(|ch| !ch.is_control() && !('\u{e000}'..='\u{f8ff}').contains(ch))
        .collect()
}

fn unix_now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
