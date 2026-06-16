//! 后台事件消息路由。
//!
//! 后端 worker 完成命令后会把事件重新包装成 `Message::BackendEventReceived`。
//! 这样后台结果和用户操作一样经过状态入口，状态变化路径保持统一。

use super::super::{AppUpdateOutcome, Message};
use crate::core::CoreState;

impl CoreState {
    /// 分发后端事件消息。
    pub(super) fn dispatch_backend_message(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::BackendEventReceived(event) => self.apply_backend_event(event),
            _ => unreachable!("非后台事件消息不应进入后台事件路由"),
        }
    }
}
