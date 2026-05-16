//! SSH 隧道运行态操作。

use crate::model::{HostId, SessionId, SessionKind, TunnelRule, TunnelRuntimeState, TunnelStatus};

use super::SessionManager;

impl SessionManager {
    /// 标记隧道开始启动。
    pub fn start_tunnel(
        &mut self,
        rule: &TunnelRule,
        host_id: Option<HostId>,
        started_at_unix_secs: u64,
    ) {
        self.upsert_tunnel(TunnelRuntimeState {
            rule_name: rule.name.clone(),
            host_id,
            status: TunnelStatus::Starting,
            started_at_unix_secs: Some(started_at_unix_secs),
            last_error: None,
        });
    }

    /// 标记隧道已经进入运行态。
    pub fn mark_tunnel_running(&mut self, rule_name: &str) -> bool {
        self.update_tunnel(rule_name, |state| {
            state.status = TunnelStatus::Running;
            state.last_error = None;
        })
    }

    /// 标记隧道停止。
    pub fn stop_tunnel(&mut self, rule_name: &str) -> bool {
        self.update_tunnel(rule_name, |state| {
            state.status = TunnelStatus::Stopped;
            state.started_at_unix_secs = None;
        })
    }

    /// 标记隧道失败并记录错误。
    pub fn fail_tunnel(&mut self, rule_name: &str, reason: impl Into<String>) -> bool {
        let reason = reason.into();

        self.update_tunnel(rule_name, |state| {
            state.status = TunnelStatus::Failed;
            state.last_error = Some(reason.clone());
        })
    }

    /// 按会话标签页标记隧道失败。
    pub fn fail_tunnel_for_session(
        &mut self,
        session_id: SessionId,
        reason: impl Into<String>,
    ) -> bool {
        let reason = reason.into();
        let Some(rule_name) = self.tabs.iter().find_map(|tab| match &tab.kind {
            SessionKind::Tunnel { rule_name } if tab.id == session_id => Some(rule_name.clone()),
            _ => None,
        }) else {
            return false;
        };

        self.fail_tunnel(&rule_name, reason)
    }

    /// 按后端事件同步隧道运行态。
    pub fn set_tunnel_status(&mut self, rule_name: &str, status: TunnelStatus) -> bool {
        self.update_tunnel(rule_name, |state| {
            if matches!(status, TunnelStatus::Stopped) {
                state.started_at_unix_secs = None;
            }
            if !matches!(status, TunnelStatus::Failed) {
                state.last_error = None;
            }
            state.status = status;
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TunnelKind;
    use uuid::Uuid;

    fn host_id() -> HostId {
        HostId(Uuid::new_v4())
    }

    fn session_id() -> SessionId {
        SessionId(Uuid::new_v4())
    }

    fn tunnel_rule(name: &str) -> TunnelRule {
        TunnelRule {
            name: name.to_owned(),
            kind: TunnelKind::Local,
            bind_host: "127.0.0.1".to_owned(),
            bind_port: 15432,
            target_host: "10.0.0.5".to_owned(),
            target_port: 5432,
            auto_start: false,
        }
    }

    #[test]
    fn tunnel_runtime_state_moves_through_start_running_stop() {
        let mut sessions = SessionManager::default();
        let rule = tunnel_rule("local-db");
        let host_id = host_id();

        sessions.start_tunnel(&rule, Some(host_id), 1_700_000_000);

        assert_eq!(sessions.tunnel_runtime_count(), 1);
        assert_eq!(sessions.tunnels[0].host_id, Some(host_id));
        assert!(matches!(sessions.tunnels[0].status, TunnelStatus::Starting));
        assert_eq!(
            sessions.tunnels[0].started_at_unix_secs,
            Some(1_700_000_000)
        );

        assert!(sessions.mark_tunnel_running("local-db"));
        assert!(matches!(sessions.tunnels[0].status, TunnelStatus::Running));

        assert!(sessions.stop_tunnel("local-db"));
        assert!(matches!(sessions.tunnels[0].status, TunnelStatus::Stopped));
        assert_eq!(sessions.tunnels[0].started_at_unix_secs, None);
    }

    #[test]
    fn tunnel_runtime_state_records_failure_reason() {
        let mut sessions = SessionManager::default();
        let rule = tunnel_rule("local-db");

        sessions.start_tunnel(&rule, None, 10);

        assert!(sessions.fail_tunnel("local-db", "bind failed"));
        assert!(matches!(sessions.tunnels[0].status, TunnelStatus::Failed));
        assert_eq!(
            sessions.tunnels[0].last_error.as_deref(),
            Some("bind failed")
        );
        assert!(!sessions.fail_tunnel("missing", "not found"));
    }

    #[test]
    fn fail_tunnel_for_session_updates_matching_tunnel_only() {
        let mut sessions = SessionManager::default();
        let rule = tunnel_rule("local-db");
        let session_id = session_id();
        let host_id = host_id();

        sessions.open_tunnel_tab(session_id, host_id, &rule);
        sessions.start_tunnel(&rule, Some(host_id), 10);

        assert!(sessions.fail_tunnel_for_session(session_id, "bind failed"));
        assert!(matches!(sessions.tunnels[0].status, TunnelStatus::Failed));
        assert_eq!(
            sessions.tunnels[0].last_error.as_deref(),
            Some("bind failed")
        );
        assert!(!sessions.fail_tunnel_for_session(SessionId(Uuid::new_v4()), "missing"));
    }

    #[test]
    fn starting_same_tunnel_replaces_previous_runtime_state() {
        let mut sessions = SessionManager::default();
        let rule = tunnel_rule("local-db");

        sessions.start_tunnel(&rule, None, 10);
        sessions.mark_tunnel_running("local-db");
        sessions.start_tunnel(&rule, None, 20);

        assert_eq!(sessions.tunnel_runtime_count(), 1);
        assert!(matches!(sessions.tunnels[0].status, TunnelStatus::Starting));
        assert_eq!(sessions.tunnels[0].started_at_unix_secs, Some(20));
    }

    #[test]
    fn set_tunnel_status_synchronizes_backend_status() {
        let mut sessions = SessionManager::default();
        let rule = tunnel_rule("local-db");

        sessions.start_tunnel(&rule, None, 10);

        assert!(sessions.set_tunnel_status("local-db", TunnelStatus::Running));
        assert!(matches!(sessions.tunnels[0].status, TunnelStatus::Running));
        assert!(sessions.set_tunnel_status("local-db", TunnelStatus::Stopped));
        assert_eq!(sessions.tunnels[0].started_at_unix_secs, None);
        assert!(!sessions.set_tunnel_status("missing", TunnelStatus::Running));
    }
}
