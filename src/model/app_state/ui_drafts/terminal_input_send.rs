//! 终端输入发送处理。
//!
//! 发送是终端输入草稿的提交点：这里校验会话类型、记录历史、排队后端输入命令，并在本地
//! 终端场景做即时回显。UI 层不需要知道这些分支。

use crate::backend::BackendCommand;
use crate::model::{SessionId, SessionKind};

use super::super::{AppState, AppUpdateOutcome};
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
        // 先从 session tab 读取会话类型和 host_id；terminal tab 只负责缓冲区，不存业务语义。
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
        // 会话必须处于可交互状态，避免断线后继续把输入排入后端队列。
        if !tab.can_accept_terminal_input() {
            return AppUpdateOutcome {
                error: Some("当前 Shell 会话不可交互，请重新连接后再发送输入".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        // 本地终端没有主机；远程 shell 必须带 host_id，历史和权限判断都依赖它。
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
        // 远程 shell 禁止发送空命令；本地终端允许空输入，用于模拟按下 Enter。
        if trimmed.is_empty() && !matches!(tab.kind, SessionKind::LocalShell) {
            return AppUpdateOutcome {
                error: Some("终端输入不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        if !trimmed.is_empty() {
            // 只记录非空命令，避免历史里充满单独回车。
            record_terminal_input_history(&mut self.storage, host_id, input.clone());
        }

        // 后端 shell 接收的是完整行输入，所以在状态层统一追加换行。
        self.backend_commands.push(BackendCommand::SendShellInput {
            session_id,
            input: format!("{input}\n"),
        });
        // 本地终端先做 UI 回显，远程终端等待真实输出事件。
        echo_local_terminal_input(&mut self.terminal, session_id, &tab.kind, &trimmed, &input);
        self.ui.clear_terminal_input(session_id);

        AppUpdateOutcome {
            state_changed: true,
            queued_backend_commands: 1,
            ..AppUpdateOutcome::default()
        }
    }
}
