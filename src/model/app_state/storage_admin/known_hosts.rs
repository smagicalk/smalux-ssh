//! Known Hosts 记录管理。
//!
//! Known Hosts 属于安全决策数据。状态层只提供明确动作：信任指定记录或删除指定记录；
//! 不在这里做自动信任，也不根据 UI 文本推断安全状态。

use super::super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 将指定 Known Hosts 记录标记为可信。
    pub(in crate::model::app_state) fn trust_known_host(
        &mut self,
        host: &str,
        port: u16,
    ) -> AppUpdateOutcome {
        // host + port 是当前的查找键；算法和指纹保留在记录里供 UI 展示。
        if let Some(entry) = self
            .storage
            .known_hosts
            .iter_mut()
            .find(|entry| entry.host == host && entry.port == port)
        {
            if entry.trusted {
                // 已经可信时不产生状态变更，避免无意义刷新。
                return AppUpdateOutcome::default();
            }

            entry.trusted = true;
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            AppUpdateOutcome {
                error: Some(format!("找不到 Known Hosts 记录：{host}:{port}")),
                ..AppUpdateOutcome::default()
            }
        }
    }

    /// 删除一个 Known Hosts 记录。
    pub(in crate::model::app_state) fn remove_known_host(
        &mut self,
        host: &str,
        port: u16,
    ) -> AppUpdateOutcome {
        // 删除允许用户撤销错误信任，下一次连接会重新触发密钥校验流程。
        if self.storage.remove_known_host(host, port) {
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            AppUpdateOutcome {
                error: Some(format!("找不到 Known Hosts 记录：{host}:{port}")),
                ..AppUpdateOutcome::default()
            }
        }
    }
}
