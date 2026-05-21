//! Known Hosts 记录管理。

use super::super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 将指定 Known Hosts 记录标记为可信。
    pub(in crate::model::app_state) fn trust_known_host(
        &mut self,
        host: &str,
        port: u16,
    ) -> AppUpdateOutcome {
        if let Some(entry) = self
            .storage
            .known_hosts
            .iter_mut()
            .find(|entry| entry.host == host && entry.port == port)
        {
            if entry.trusted {
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
