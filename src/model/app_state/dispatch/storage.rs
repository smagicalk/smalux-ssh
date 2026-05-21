//! 本地存储管理消息路由。

use super::super::{AppState, AppUpdateOutcome, Message};

impl AppState {
    pub(super) fn dispatch_storage_message(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::RemoveCredential { name } => self.remove_credential(&name),
            Message::TrustKnownHost { host, port } => self.trust_known_host(&host, port),
            Message::RemoveKnownHost { host, port } => self.remove_known_host(&host, port),
            _ => unreachable!("非存储管理消息不应进入存储管理路由"),
        }
    }
}
