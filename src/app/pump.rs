//! 后端输出刷新泵。
//!
//! 交互式 PTY 的输出可能晚于用户按下回车到达，因此需要一个轻量定时器
//! 定期抽取执行器里已经读到的事件，并同步到 Slint 窗口。

#[path = "pump/drain.rs"]
mod drain;
#[cfg(test)]
#[path = "pump/tests.rs"]
mod tests;

use slint::{ComponentHandle, Timer, TimerMode};

use super::projection::sync_terminal_pane;
use super::{AppWindow, SharedAppState};
use drain::enqueue_drain_commands;

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
