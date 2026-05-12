//! 会话标签页的打开、状态更新和关闭操作。

use crate::model::{HostId, SessionId, SessionKind, SessionStatus, SessionTab, TunnelRule};

use super::SessionManager;

impl SessionManager {
    /// 打开一个新的交互式 Shell 标签页。
    pub fn open_shell_tab(&mut self, id: SessionId, host_id: HostId, title: impl Into<String>) {
        self.push_tab(SessionTab {
            id,
            host_id: Some(host_id),
            kind: SessionKind::Shell,
            title: title.into(),
            status: SessionStatus::Created,
        });
    }

    /// 打开一个远程命令执行标签页。
    pub fn open_remote_command_tab(
        &mut self,
        id: SessionId,
        host_id: HostId,
        command: impl Into<String>,
    ) {
        let command = command.into();

        self.push_tab(SessionTab {
            id,
            host_id: Some(host_id),
            kind: SessionKind::RemoteCommand {
                command: command.clone(),
            },
            title: command,
            status: SessionStatus::Created,
        });
    }

    /// 打开一个隧道管理标签页。
    pub fn open_tunnel_tab(&mut self, id: SessionId, host_id: HostId, rule: &TunnelRule) {
        self.push_tab(SessionTab {
            id,
            host_id: Some(host_id),
            kind: SessionKind::Tunnel {
                rule_name: rule.name.clone(),
            },
            title: rule.display_endpoint(),
            status: SessionStatus::Created,
        });
    }

    /// 更新标签页状态。
    pub fn set_status(&mut self, id: SessionId, status: SessionStatus) -> bool {
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == id) {
            tab.status = status;
            true
        } else {
            false
        }
    }

    /// 关闭标签页，并同步活动连接索引。
    pub fn close_tab(&mut self, id: SessionId) -> bool {
        let before = self.tabs.len();
        self.tabs.retain(|tab| tab.id != id);
        self.active.retain(|active_id| *active_id != id);

        if self.active_tab == Some(id) {
            self.active_tab = self.tabs.last().map(|tab| tab.id);
        }

        before != self.tabs.len()
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
    fn opening_shell_tab_tracks_active_session_and_tab() {
        let mut sessions = SessionManager::default();
        let id = session_id();
        let host_id = host_id();

        sessions.open_shell_tab(id, host_id, "production");

        assert_eq!(sessions.active_count(), 1);
        assert_eq!(sessions.tab_count(), 1);
        assert_eq!(sessions.active_tab, Some(id));
        assert_eq!(sessions.tabs[0].host_id, Some(host_id));
        assert!(matches!(sessions.tabs[0].kind, SessionKind::Shell));
        assert!(matches!(sessions.tabs[0].status, SessionStatus::Created));
    }

    #[test]
    fn opening_remote_command_tab_sets_command_title_and_kind() {
        let mut sessions = SessionManager::default();
        let id = session_id();

        sessions.open_remote_command_tab(id, host_id(), "uptime");

        assert_eq!(sessions.tabs[0].title, "uptime");
        assert!(matches!(
            &sessions.tabs[0].kind,
            SessionKind::RemoteCommand { command } if command == "uptime"
        ));
    }

    #[test]
    fn opening_tunnel_tab_uses_rule_name_and_endpoint_title() {
        let mut sessions = SessionManager::default();
        let id = session_id();
        let rule = tunnel_rule("local-db");

        sessions.open_tunnel_tab(id, host_id(), &rule);

        assert_eq!(sessions.tab_count(), 1);
        assert_eq!(sessions.tabs[0].title, "L 127.0.0.1:15432 -> 10.0.0.5:5432");
        assert!(matches!(
            &sessions.tabs[0].kind,
            SessionKind::Tunnel { rule_name } if rule_name == "local-db"
        ));
    }

    #[test]
    fn set_status_updates_existing_tab_only() {
        let mut sessions = SessionManager::default();
        let id = session_id();

        sessions.open_shell_tab(id, host_id(), "production");

        assert!(sessions.set_status(id, SessionStatus::Connected));
        assert!(matches!(sessions.tabs[0].status, SessionStatus::Connected));
        assert!(!sessions.set_status(
            session_id(),
            SessionStatus::Failed {
                reason: "missing".to_owned()
            }
        ));
    }

    #[test]
    fn close_tab_removes_active_session_and_selects_previous_tab() {
        let mut sessions = SessionManager::default();
        let first_id = session_id();
        let second_id = session_id();

        sessions.open_shell_tab(first_id, host_id(), "first");
        sessions.open_shell_tab(second_id, host_id(), "second");

        assert!(sessions.close_tab(second_id));
        assert_eq!(sessions.active_count(), 1);
        assert_eq!(sessions.tab_count(), 1);
        assert_eq!(sessions.active_tab, Some(first_id));
        assert!(!sessions.close_tab(second_id));
    }
}
