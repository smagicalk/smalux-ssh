//! 应用消息处理结果。

/// 应用消息处理结果。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppUpdateOutcome {
    pub state_changed: bool,
    pub queued_backend_commands: usize,
    pub worker_command: Option<crate::backend::BackendCommand>,
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
