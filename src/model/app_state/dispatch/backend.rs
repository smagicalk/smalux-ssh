//! 后台事件消息路由。

use super::super::{AppState, AppUpdateOutcome, Message};

impl AppState {
    pub(super) fn dispatch_backend_message(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::BackendEventReceived(event) => self.apply_backend_event(event),
            _ => unreachable!("非后台事件消息不应进入后台事件路由"),
        }
    }
}
