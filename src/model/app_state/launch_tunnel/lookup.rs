//! 隧道标签页查询。

use crate::model::{SessionId, SessionKind};

use super::super::AppState;

impl AppState {
    /// 判断同名隧道标签是否已经打开，避免后端同名隧道互相覆盖。
    pub(super) fn has_open_tunnel_tab(&self, rule_name: &str) -> bool {
        self.sessions.tabs.iter().any(|tab| {
            matches!(
                &tab.kind,
                SessionKind::Tunnel {
                    rule_name: existing_rule_name,
                } if existing_rule_name == rule_name
            )
        })
    }

    /// 判断停止命令是否来自对应的隧道标签页。
    pub(super) fn tunnel_stop_target_matches_tab(
        &self,
        session_id: SessionId,
        rule_name: &str,
    ) -> bool {
        self.sessions.tabs.iter().any(|tab| {
            tab.id == session_id
                && matches!(
                    &tab.kind,
                    SessionKind::Tunnel {
                        rule_name: tab_rule_name,
                    } if tab_rule_name == rule_name
                )
        })
    }
}
