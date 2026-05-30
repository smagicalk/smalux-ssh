//! 会话标签页快照和生命周期状态。
//!
//! 这里定义“用户看到的标签页”和后端连接状态，不包含真实 socket/PTY 句柄。这样会话状态
//! 可以被持久化、测试和 UI 投影复用。

use serde::{Deserialize, Serialize};

use crate::{CommandHistoryId, HostId, SessionId};

/// UI 中打开的会话标签页。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTab {
    /// 标签页稳定 ID，同时也是后端命令和终端缓冲的关联键。
    pub id: SessionId,
    /// 远程会话关联主机；本地终端为 None。
    pub host_id: Option<HostId>,
    /// 标签页功能类型。
    pub kind: SessionKind,
    /// 用户可见标题。
    pub title: String,
    /// 生命周期状态。
    pub status: SessionStatus,
}

impl SessionTab {
    /// 判断该标签页当前是否可以接收终端交互输入。
    pub fn can_accept_terminal_input(&self) -> bool {
        matches!(self.kind, SessionKind::LocalShell)
            || (matches!(self.kind, SessionKind::Shell)
                && matches!(self.status, SessionStatus::Connected))
    }

    /// 判断该标签页是否可以由用户主动重新连接。
    pub fn can_reconnect_shell(&self) -> bool {
        matches!(self.kind, SessionKind::Shell)
            && matches!(
                self.status,
                SessionStatus::Disconnected | SessionStatus::Failed { .. }
            )
    }
}

/// 会话标签页承载的功能类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionKind {
    /// 本机 shell/PowerShell，不依赖 Host。
    LocalShell,
    /// 远程交互式 SSH shell。
    Shell,
    /// 单次远程命令，可能关联一条命令历史。
    RemoteCommand {
        command: String,
        history_id: Option<CommandHistoryId>,
    },
    /// SFTP 浏览标签。
    Sftp,
    /// SSH 隧道标签，按规则名关联隧道配置。
    Tunnel { rule_name: String },
}

/// SSH 会话生命周期状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    /// 刚创建，尚未提交后端连接命令。
    Created,
    /// 正在建立网络连接。
    Connecting,
    /// 正在认证。
    Authenticating,
    /// 已连接或本地终端已可用。
    Connected,
    /// 单次远程命令正在执行。
    RunningCommand,
    /// 用户主动重连中的远程 shell。
    Reconnecting,
    /// 正常断开，属于终态。
    Disconnected,
    /// 失败断开，属于终态。
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
    fn shell_reconnect_acceptance_depends_on_kind_and_terminal_status() {
        let host_id = HostId(Uuid::new_v4());
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
        let failed_shell = SessionTab {
            status: SessionStatus::Failed {
                reason: "network".to_owned(),
            },
            ..connected_shell.clone()
        };
        let remote_command = SessionTab {
            kind: SessionKind::RemoteCommand {
                command: "uptime".to_owned(),
                history_id: None,
            },
            status: SessionStatus::Disconnected,
            ..connected_shell.clone()
        };

        assert!(!connected_shell.can_reconnect_shell());
        assert!(disconnected_shell.can_reconnect_shell());
        assert!(failed_shell.can_reconnect_shell());
        assert!(!remote_command.can_reconnect_shell());
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
