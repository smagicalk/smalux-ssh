//! 会话标签页快照和生命周期状态。

use serde::{Deserialize, Serialize};

use super::{HostId, SessionId};

/// UI 中打开的会话标签页。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionTab {
    pub id: SessionId,
    pub host_id: Option<HostId>,
    pub kind: SessionKind,
    pub title: String,
    pub status: SessionStatus,
}

/// 会话标签页承载的功能类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionKind {
    Shell,
    RemoteCommand { command: String },
    Sftp,
    Tunnel { rule_name: String },
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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn session_tab_round_trips_remote_command_state() {
        let host_id = HostId(Uuid::new_v4());
        let tab = SessionTab {
            id: SessionId(Uuid::new_v4()),
            host_id: Some(host_id),
            kind: SessionKind::RemoteCommand {
                command: "uptime".to_owned(),
            },
            title: "uptime".to_owned(),
            status: SessionStatus::RunningCommand,
        };

        let encoded = toml::to_string(&tab).expect("会话标签页应该可以序列化为 TOML");
        let decoded: SessionTab =
            toml::from_str(&encoded).expect("会话标签页应该可以从 TOML 反序列化");

        assert_eq!(decoded, tab);
    }
}
