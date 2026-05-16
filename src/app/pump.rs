//! 后端输出刷新泵。
//!
//! 交互式 PTY 的输出可能晚于用户按下回车到达，因此需要一个轻量定时器
//! 定期抽取执行器里已经读到的事件，并同步到 Slint 窗口。

use slint::{ComponentHandle, Timer, TimerMode};

use crate::backend::BackendCommand;

use super::projection::sync_terminal_pane;
use super::{AppWindow, SharedAppState};

const BACKEND_PUMP_INTERVAL: std::time::Duration = std::time::Duration::from_millis(16);

/// 启动后端输出刷新定时器。
pub(super) fn start_backend_pump(window: &AppWindow, state: SharedAppState) {
    let timer = Timer::default();
    let weak = window.as_weak();

    timer.start(TimerMode::Repeated, BACKEND_PUMP_INTERVAL, move || {
        let Some(window) = weak.upgrade() else {
            return;
        };

        let session_ids = {
            let state = state.borrow();
            state.sessions.interactive_shell_tab_ids()
        };

        let changed = {
            let mut state = state.borrow_mut();
            enqueue_drain_commands(&mut state, session_ids);
            state.drain_backend_queue_with_executor().changed()
        };

        if changed {
            sync_terminal_pane(&window, &state.borrow());
        }
    });

    // Slint 的 Timer 停止于 Drop。泄漏一个应用生命周期定时器比把它塞进
    // 可克隆 AppState 更清晰，窗口退出后进程生命周期会回收它。
    std::mem::forget(timer);
}

fn enqueue_drain_commands(
    state: &mut crate::model::AppState,
    session_ids: impl IntoIterator<Item = crate::model::SessionId>,
) {
    for session_id in session_ids {
        state
            .backend_commands
            .push(BackendCommand::DrainSessionOutput { session_id });
    }
}

#[cfg(test)]
mod tests {
    use super::enqueue_drain_commands;
    use crate::backend::BackendCommand;
    use crate::model::{AppState, DEFAULT_LOCAL_TERMINAL_TITLE, SessionStatus};
    use uuid::Uuid;

    fn session_id() -> crate::model::SessionId {
        crate::model::SessionId(Uuid::new_v4())
    }

    #[test]
    fn pump_drain_queue_targets_only_connected_interactive_shells() {
        let mut state = AppState::default();
        let local_id = session_id();
        let shell_id = session_id();
        let remote_command_id = session_id();

        state
            .sessions
            .open_local_shell_tab(local_id, DEFAULT_LOCAL_TERMINAL_TITLE);
        state
            .sessions
            .open_shell_tab(shell_id, crate::model::HostId(Uuid::new_v4()), "ssh");
        state.sessions.open_remote_command_tab(
            remote_command_id,
            crate::model::HostId(Uuid::new_v4()),
            "uptime",
        );
        assert!(
            state
                .sessions
                .set_status(shell_id, SessionStatus::Connected)
        );
        assert!(
            state
                .sessions
                .set_status(remote_command_id, SessionStatus::RunningCommand)
        );

        let session_ids = state.sessions.interactive_shell_tab_ids();
        enqueue_drain_commands(&mut state, session_ids);

        assert_eq!(
            state.backend_commands.drain(),
            vec![
                BackendCommand::DrainSessionOutput {
                    session_id: local_id
                },
                BackendCommand::DrainSessionOutput {
                    session_id: shell_id
                }
            ]
        );
    }
}
