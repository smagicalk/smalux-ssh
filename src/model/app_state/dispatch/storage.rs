//! 本地存储管理消息路由。
//!
//! 这里处理已经保存的数据的确认删除和安全资产维护。真正的落盘由 UI Adapter
//! 在状态变化后统一调用 `persist_storage`，领域函数只负责修改内存快照。

use super::super::{AppState, AppUpdateOutcome, Message};

impl AppState {
    /// 分发存储管理消息。
    ///
    /// 删除主机和分组采用 request/confirm 两步：request 只记录待确认目标，
    /// confirm 才真正修改存储，方便 UI 展示确认弹窗。
    pub(super) fn dispatch_storage_message(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::ConfirmRemoveHost => self.confirm_remove_host(),
            Message::ConfirmRemoveGroup => self.confirm_remove_group(),
            Message::RemoveCredential { name } => self.remove_credential(&name),
            Message::TrustKnownHost { host, port } => self.trust_known_host(&host, port),
            Message::RemoveKnownHost { host, port } => self.remove_known_host(&host, port),
            _ => unreachable!("非存储管理消息不应进入存储管理路由"),
        }
    }
}
