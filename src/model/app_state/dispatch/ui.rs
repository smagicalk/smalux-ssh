//! UI 草稿与工作区界面消息入口路由。
//!
//! 这里只保留 UI 类消息的二级入口，具体分支按主机表单、工作区/设置和终端输入拆到
//! sibling 模块中。它仍在核心状态层中，目的是让新 UI 能复用同一套交互状态和测试。

use super::super::{AppState, AppUpdateOutcome, Message};

impl AppState {
    /// 分发纯 UI 状态消息。
    ///
    /// `MessageDispatchTarget` 已经保证只会把 UI 类消息送到这里；最后的
    /// `unreachable!` 用来捕获新增消息未分类或分类错误。
    pub(super) fn dispatch_ui_message(&mut self, message: Message) -> AppUpdateOutcome {
        self.try_dispatch_quick_host_ui_message(&message)
            .or_else(|| self.try_dispatch_workspace_ui_message(&message))
            .or_else(|| self.try_dispatch_terminal_ui_message(&message))
            .unwrap_or_else(|| unreachable!("非 UI 消息不应进入 UI 路由"))
    }
}
