//! 本地终端标签页确保逻辑。
//!
//! 本地终端和远程 SSH shell 共用同一套 session/tab/terminal 状态。这个模块负责在用户
//! 新建本地终端时同时确保“会话标签”和“终端缓冲区标签”存在，并把 PTY 启动请求排入后端。

use uuid::Uuid;

use crate::backend::{BackendCommand, PtyRequest};
use crate::core::CoreState;
use crate::model::{SessionId, SessionKind, WorkspacePage};
use crate::terminal::TerminalTabState;

use super::super::{AppState, AppUpdateOutcome};

pub(in crate::model::app_state) fn ensure_local_terminal_tab(
    state: &mut CoreState,
    session_id: SessionId,
) -> bool {
    let title = local_terminal_title(state, session_id);
    let had_session = state.sessions.tabs.iter().any(|tab| tab.id == session_id);
    if !had_session {
        // session tab 负责应用层语义：标题、会话类型、连接状态、当前激活项。
        state
            .sessions
            .open_local_shell_tab(session_id, title.clone());
    }

    let had_terminal = state
        .terminal
        .tabs
        .iter()
        .any(|tab| tab.session_id == session_id);
    if !had_terminal {
        // terminal tab 负责终端缓冲区、尺寸和滚动状态，和 session tab 分开维护。
        state
            .terminal
            .open_tab(TerminalTabState::new(session_id, title));
    }

    !had_session || !had_terminal
}

fn local_terminal_title(state: &CoreState, session_id: SessionId) -> String {
    if let Some(tab) = state.sessions.tabs.iter().find(|tab| tab.id == session_id) {
        // 已存在 session 时沿用原标题，避免重复 ensure 改变用户看到的 tab 名称。
        return tab.title.clone();
    }

    let existing_titles: Vec<&str> = state
        .sessions
        .tabs
        .iter()
        .filter(|tab| matches!(tab.kind, SessionKind::LocalShell))
        .map(|tab| tab.title.as_str())
        .collect();
    if !existing_titles.contains(&crate::model::DEFAULT_LOCAL_TERMINAL_TITLE) {
        return crate::model::DEFAULT_LOCAL_TERMINAL_TITLE.to_owned();
    }

    // 多开本地终端时生成稳定的递增标题：本地终端、本地终端 2、本地终端 3...
    (2..)
        .map(|index| format!("{} {}", crate::model::DEFAULT_LOCAL_TERMINAL_TITLE, index))
        .find(|title| !existing_titles.contains(&title.as_str()))
        .expect("本地终端标题生成应始终能找到可用编号")
}

impl CoreState {
    /// 新建本地终端的稳定核心入口。
    #[cfg_attr(not(feature = "desktop"), allow(dead_code))]
    pub(crate) fn open_local_terminal_action(&mut self) -> AppUpdateOutcome {
        self.open_local_terminal()
    }

    /// 新建一个本地终端标签页，并把本地 PTY 启动请求排入后端队列。
    pub(in crate::model::app_state) fn open_local_terminal(&mut self) -> AppUpdateOutcome {
        let session_id = SessionId(Uuid::new_v4());
        ensure_local_terminal_tab(self, session_id);
        // PTY 需要知道当前终端尺寸。若终端 tab 尚未投影出尺寸，则使用默认尺寸启动。
        let pty = self
            .terminal
            .tabs
            .iter()
            .find(|tab| tab.session_id == session_id)
            .map(|tab| PtyRequest::xterm(tab.size))
            .unwrap_or_else(|| PtyRequest::xterm(crate::terminal::TerminalSize::default()));

        self.backend_commands
            .push(BackendCommand::OpenLocalShell { session_id, pty });

        AppUpdateOutcome {
            state_changed: true,
            queued_backend_commands: 1,
            ..AppUpdateOutcome::default()
        }
    }
}

impl AppState {
    /// 桌面端打开本地终端后切换到终端页，并保持主机侧栏展开。
    pub(in crate::model::app_state) fn open_local_terminal(&mut self) -> AppUpdateOutcome {
        let outcome = self.core.open_local_terminal();
        if outcome.changed() {
            self.ui.workspace.active_page = WorkspacePage::Terminal;
        }
        let hosts_panel_changed = self.ui.workspace.hosts_panel_collapsed;
        if hosts_panel_changed {
            self.ui.workspace.set_hosts_panel_collapsed(false);
        }

        AppUpdateOutcome {
            state_changed: outcome.state_changed || hosts_panel_changed,
            ..outcome
        }
    }
}
