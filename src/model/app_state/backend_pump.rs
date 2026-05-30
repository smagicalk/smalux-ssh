//! 后端命令队列执行泵。
//!
//! `AppState` 不直接持有 SSH、PTY 或 SFTP 连接，它只把用户意图转换成
//! `BackendCommand` 放进队列。执行泵负责把这些命令交给后端执行器，并把后端返回的
//! `BackendEvent` 重新归约回纯状态。这样 UI 层可以完全不知道命令是同步执行、后台线程
//! 执行，还是以后换成异步 runtime 执行。
//!
//! 当前保留了两种入口：
//! - `drain_backend_queue`：测试或简单运行模式使用，一次性把队列 drain 掉。
//! - `next_backend_command_for_worker` + `apply_backend_command_result`：真实 UI worker 使用，
//!   主线程只派发下一条可执行命令，worker 完成后再把结果投回状态层。

use crate::backend::{BackendCommand, BackendEvent, BackendExecutionError, BackendExecutor};

mod command_eligibility;
mod execution_failure;
mod host_keys;
mod pending;
mod stale_commands;
mod transfers;

use super::{AppState, AppUpdateOutcome};
use execution_failure::handle_backend_execution_error;
use transfers::{FailedTransfer, failed_transfer_for_command};

/// 后端 worker 执行完一条命令后回传给 UI 状态层的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendCommandResult {
    /// worker 实际执行的命令。结果回来时仍保留原命令，方便状态层判断失败语义。
    pub command: BackendCommand,
    /// 后端执行结果。成功时是一组事件，失败时保留具体错误类型以便映射到 UI 状态。
    pub result: Result<Vec<BackendEvent>, BackendExecutionError>,
}

impl BackendCommandResult {
    pub fn new(
        command: BackendCommand,
        result: Result<Vec<BackendEvent>, BackendExecutionError>,
    ) -> Self {
        Self { command, result }
    }
}

impl AppState {
    /// 将当前队列中的后端命令交给执行器，并把返回事件归约到状态。
    pub fn drain_backend_queue(
        &mut self,
        executor: &mut (impl BackendExecutor + ?Sized),
    ) -> AppUpdateOutcome {
        let mut outcome = AppUpdateOutcome::default();

        while let Some(command) = self.backend_commands.pop_front() {
            // 命令入队后，用户可能已经关闭标签页、切换会话或取消传输。
            // 执行前必须重新判定一次，避免对已经失效的会话继续做 IO。
            if !self.can_execute_backend_command(&command) {
                let skip_outcome = self.skip_stale_backend_command(&command);
                outcome.state_changed |= skip_outcome.state_changed;
                outcome.applied_backend_events += skip_outcome.applied_backend_events;
                continue;
            }

            // 执行失败时需要知道命令所属会话，以及它是否对应一个可见的 SFTP 传输。
            // 这些信息从命令本身抽取，避免后端错误类型反向耦合 UI 传输列表。
            let session_id = command.session_id();
            let failed_transfer = failed_transfer_for_command(&command);
            let events = match executor.execute(command) {
                Ok(events) => events,
                Err(error) => {
                    let error_outcome = self.apply_backend_execution_result(
                        session_id,
                        failed_transfer,
                        Err(error),
                    );
                    outcome.state_changed |= error_outcome.state_changed;
                    outcome.executed_backend_commands += error_outcome.executed_backend_commands;
                    outcome.applied_backend_events += error_outcome.applied_backend_events;
                    outcome.error = error_outcome.error;
                    // 同一会话后续命令通常依赖当前命令成功；失败后停止本轮 drain，
                    // 把错误交给上层显示，避免继续扩大状态污染。
                    break;
                }
            };

            outcome.executed_backend_commands += 1;
            for event in events {
                let event_outcome = self.apply_backend_event(event);
                outcome.state_changed |= event_outcome.state_changed;
                outcome.applied_backend_events += event_outcome.applied_backend_events;
            }
        }

        outcome
    }

    /// 从 UI 状态队列取出下一条仍可执行的后端命令，交给后台 worker 执行。
    pub fn next_backend_command_for_worker(&mut self) -> AppUpdateOutcome {
        let mut outcome = AppUpdateOutcome::default();

        while let Some(command) = self.backend_commands.pop_front() {
            if self.can_execute_backend_command(&command) {
                // worker 入口一次只返回一条命令，主线程不会在这里阻塞执行真实 IO。
                outcome.worker_command = Some(command);
                return outcome;
            }

            // 失效命令虽然不执行，但仍需要做状态收尾，例如清除 loading 或标记传输失败。
            let skip_outcome = self.skip_stale_backend_command(&command);
            outcome.state_changed |= skip_outcome.state_changed;
            outcome.applied_backend_events += skip_outcome.applied_backend_events;
        }

        outcome
    }

    /// 将后台 worker 的执行结果归约回 UI 状态。
    pub fn apply_backend_command_result(
        &mut self,
        command: BackendCommand,
        result: Result<Vec<BackendEvent>, BackendExecutionError>,
    ) -> AppUpdateOutcome {
        let session_id = command.session_id();
        let failed_transfer = failed_transfer_for_command(&command);
        self.apply_backend_execution_result(session_id, failed_transfer, result)
    }

    fn apply_backend_execution_result(
        &mut self,
        session_id: crate::model::SessionId,
        failed_transfer: Option<FailedTransfer>,
        result: Result<Vec<BackendEvent>, BackendExecutionError>,
    ) -> AppUpdateOutcome {
        let mut outcome = AppUpdateOutcome::default();

        match result {
            Ok(events) => {
                // 后端可以把一次命令拆成多个领域事件，例如连接成功后附带终端输出。
                outcome.executed_backend_commands += 1;
                for event in events {
                    let event_outcome = self.apply_backend_event(event);
                    outcome.state_changed |= event_outcome.state_changed;
                    outcome.applied_backend_events += event_outcome.applied_backend_events;
                }
            }
            Err(error) => {
                // 失败路径集中在一个模块中处理，保证连接、SFTP、传输、known_hosts 的
                // 状态清理规则保持一致。
                handle_backend_execution_error(
                    self,
                    &mut outcome,
                    session_id,
                    failed_transfer,
                    error,
                );
            }
        }

        outcome
    }
}
