//! 后端输出刷新泵。
//!
//! 交互式 PTY 的输出可能晚于用户按下回车到达，因此需要一个轻量定时器
//! 定期抽取执行器里已经读到的事件，并同步到 Slint 窗口。
//!
//! 这也是 Slint Adapter 的一部分。核心状态只维护后端命令队列和后端事件处理；
//! 定时器、线程和窗口刷新策略留在 UI Adapter 中，方便未来用别的 UI 替换。

#[path = "pump/drain.rs"]
mod drain;
#[cfg(test)]
#[path = "pump/tests.rs"]
mod tests;

use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::thread;

use slint::{ComponentHandle, Timer, TimerMode};

use crate::backend::{BackendCommand, SharedBackendExecutor};
use crate::core::CoreState;
use crate::model::{BackendCommandResult, UiState};

use super::projection::sync_terminal_pane;
use super::state::AsDesktopStateView;
use super::{AppWindow, SharedAppState};
use drain::enqueue_drain_commands;

const BACKEND_PUMP_INTERVAL: std::time::Duration = std::time::Duration::from_millis(16);

/// 启动后端输出刷新定时器。
pub(super) fn start_backend_pump(window: &AppWindow, state: SharedAppState) {
    let timer = Timer::default();
    let weak = window.as_weak();
    let worker = BackendWorker::start(state.borrow().core.backend_executor.clone());

    timer.start(TimerMode::Repeated, BACKEND_PUMP_INTERVAL, move || {
        let Some(window) = weak.upgrade() else {
            return;
        };

        let session_ids = {
            let state = state.borrow();
            state.core.sessions.interactive_shell_tab_ids()
        };

        let changed = {
            let mut state = state.borrow_mut();
            let state = &mut *state;
            // 先应用后台线程已经完成的命令结果，再补充交互式 shell 的 drain 命令。
            // 这样 UI 看到的是“旧结果已归约，新读取请求已排队”的一致状态。
            let mut changed = apply_ready_backend_results(
                &mut state.core,
                &mut state.ui,
                &worker.result_receiver,
            );
            enqueue_drain_commands(&mut state.core, session_ids);

            // 每个 tick 只向后台 worker 提交一条可执行命令，避免 UI 线程里阻塞网络
            // 或 PTY 操作，同时保留命令队列的顺序语义。
            let next = state.core.next_backend_command_for_worker();
            changed |= next.changed();
            if let Some(command) = next.worker_command {
                worker.dispatch(command);
            }

            changed
        };

        if changed {
            sync_terminal_pane(&window, state.borrow().as_desktop_state_view());
        }
    });

    // Slint 的 Timer 停止于 Drop。这里把它作为桌面 Adapter 生命周期资源保留；
    // 窗口退出后进程生命周期会回收它。
    std::mem::forget(timer);
}

/// 后台执行线程的句柄。
///
/// `CoreState` 里保存的是可替换的后端执行器；这里负责把 UI 线程里的命令队列
/// 送到后台线程执行，并把结果送回下一轮 pump。
struct BackendWorker {
    command_sender: Sender<BackendCommand>,
    result_receiver: Receiver<BackendCommandResult>,
}

impl BackendWorker {
    /// 启动单个后台 worker。
    ///
    /// 目前用一个线程顺序执行后端命令，优点是状态变化容易推理；以后如果要
    /// 并行化，应先定义清楚同一 session 内命令的顺序保证。
    fn start(executor: SharedBackendExecutor) -> Self {
        let (command_sender, command_receiver) = std::sync::mpsc::channel::<BackendCommand>();
        let (result_sender, result_receiver) = std::sync::mpsc::channel::<BackendCommandResult>();

        thread::spawn(move || {
            while let Ok(command) = command_receiver.recv() {
                let result = {
                    let mut executor = executor
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    executor.execute(command.clone())
                };
                if result_sender
                    .send(BackendCommandResult::new(command, result))
                    .is_err()
                {
                    break;
                }
            }
        });

        Self {
            command_sender,
            result_receiver,
        }
    }

    /// 把一条核心后端命令提交到后台线程。
    fn dispatch(&self, command: BackendCommand) {
        if let Err(error) = self.command_sender.send(command) {
            tracing::error!(error = %error, "提交后端命令到后台 worker 失败");
        }
    }
}

/// 把后台线程已完成的结果归约回核心状态。
///
/// 这里使用非阻塞 `try_recv`，保证 UI tick 不会等待网络或文件操作完成。
/// 返回值只表示核心状态是否变化，用于决定是否需要局部刷新终端面板。
fn apply_ready_backend_results(
    state: &mut CoreState,
    ui: &mut UiState,
    receiver: &Receiver<BackendCommandResult>,
) -> bool {
    let mut changed = false;

    loop {
        match receiver.try_recv() {
            Ok(result) => {
                let outcome = state.apply_backend_command_result(result.command, result.result);
                if let Some(error) = &outcome.error {
                    ui.set_last_error(error.clone());
                }
                changed |= outcome.changed();
            }
            Err(TryRecvError::Empty) => return changed,
            Err(TryRecvError::Disconnected) => {
                tracing::error!("后端 worker 结果通道已断开");
                return changed;
            }
        }
    }
}
