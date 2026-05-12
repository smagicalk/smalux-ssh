//! 最近连接和命令历史模型。

use serde::{Deserialize, Serialize};

use super::{CommandHistoryId, HostId};

/// 最近连接记录。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentConnection {
    pub host_id: HostId,
    pub label: String,
    pub connected_at_unix_secs: u64,
}

/// 单条命令历史。
///
/// `host_id` 为空时表示全局命令，非空时表示某个主机下的历史命令。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandHistoryItem {
    pub id: CommandHistoryId,
    pub host_id: Option<HostId>,
    pub command: String,
    pub working_directory: Option<String>,
    pub exit_code: Option<i32>,
    pub started_at_unix_secs: u64,
    pub duration_ms: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn command_history_round_trips_with_host_scope() {
        let item = CommandHistoryItem {
            id: CommandHistoryId(Uuid::new_v4()),
            host_id: Some(HostId(Uuid::new_v4())),
            command: "systemctl status sshd".to_owned(),
            working_directory: Some("/etc".to_owned()),
            exit_code: Some(0),
            started_at_unix_secs: 1_700_000_000,
            duration_ms: Some(250),
        };

        let encoded = toml::to_string(&item).expect("命令历史应该可以序列化为 TOML");
        let decoded: CommandHistoryItem =
            toml::from_str(&encoded).expect("命令历史应该可以从 TOML 反序列化");

        assert_eq!(decoded, item);
    }
}
