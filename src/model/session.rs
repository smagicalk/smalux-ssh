//! 会话标签页快照和生命周期状态。

use serde::{Deserialize, Serialize};

use super::{CommandHistoryId, HostId, SessionId};

/// UI 中打开的会话标签页。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTab {
    pub id: SessionId,
    pub host_id: Option<HostId>,
    pub kind: SessionKind,
    pub title: String,
    pub status: SessionStatus,
}

impl SessionTab {
    /// 判断该标签页当前是否可以接收终端交互输入。
    pub fn can_accept_terminal_input(&self) -> bool {
        matches!(self.kind, SessionKind::LocalShell)
            || (matches!(self.kind, SessionKind::Shell)
                && matches!(self.status, SessionStatus::Connected))
    }
}

/// 会话标签页承载的功能类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionKind {
    LocalShell,
    Shell,
    RemoteCommand {
        command: String,
        history_id: Option<CommandHistoryId>,
    },
    Sftp,
    Tunnel {
        rule_name: String,
    },
}

/// SSH 会话生命周期状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    Created,
    Connecting,
    Authenticating,
    Connected,
    RunningCommand,
    Reconnecting,
    Disconnected,
    Failed { reason: String },
}

impl SessionStatus {
    /// 判断状态是否已经进入终态，不应再被普通运行时事件拉回。
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Disconnected | Self::Failed { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn session_tab_round_trips_remote_command_state() {
        let host_id = HostId(Uuid::new_v4());
        let history_id = CommandHistoryId(Uuid::new_v4());
        let tab = SessionTab {
            id: SessionId(Uuid::new_v4()),
            host_id: Some(host_id),
            kind: SessionKind::RemoteCommand {
                command: "uptime".to_owned(),
                history_id: Some(history_id),
            },
            title: "uptime".to_owned(),
            status: SessionStatus::RunningCommand,
        };

        let encoded = toml::to_string(&tab).expect("会话标签页应该可以序列化为 TOML");
        let decoded: SessionTab =
            toml::from_str(&encoded).expect("会话标签页应该可以从 TOML 反序列化");

        assert_eq!(decoded, tab);
    }

    #[test]
    fn terminal_input_acceptance_depends_on_kind_and_status() {
        let host_id = HostId(Uuid::new_v4());
        let local = SessionTab {
            id: SessionId(Uuid::new_v4()),
            host_id: None,
            kind: SessionKind::LocalShell,
            title: "Local".to_owned(),
            status: SessionStatus::Disconnected,
        };
        let connected_shell = SessionTab {
            id: SessionId(Uuid::new_v4()),
            host_id: Some(host_id),
            kind: SessionKind::Shell,
            title: "ssh".to_owned(),
            status: SessionStatus::Connected,
        };
        let disconnected_shell = SessionTab {
            status: SessionStatus::Disconnected,
            ..connected_shell.clone()
        };
        let remote_command = SessionTab {
            kind: SessionKind::RemoteCommand {
                command: "uptime".to_owned(),
                history_id: None,
            },
            status: SessionStatus::RunningCommand,
            ..connected_shell.clone()
        };

        assert!(local.can_accept_terminal_input());
        assert!(connected_shell.can_accept_terminal_input());
        assert!(!disconnected_shell.can_accept_terminal_input());
        assert!(!remote_command.can_accept_terminal_input());
    }

    #[test]
    fn session_status_terminal_state_is_centralized() {
        assert!(!SessionStatus::Created.is_terminal());
        assert!(!SessionStatus::Connecting.is_terminal());
        assert!(!SessionStatus::Authenticating.is_terminal());
        assert!(!SessionStatus::Connected.is_terminal());
        assert!(!SessionStatus::RunningCommand.is_terminal());
        assert!(!SessionStatus::Reconnecting.is_terminal());
        assert!(SessionStatus::Disconnected.is_terminal());
        assert!(
            SessionStatus::Failed {
                reason: "network".to_owned(),
            }
            .is_terminal()
        );
    }
}
