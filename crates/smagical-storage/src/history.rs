//! 最近连接和命令历史的内存索引操作。

use smagical_core::{CommandHistoryItem, HostId, RecentConnection};

use super::{DEFAULT_COMMAND_HISTORY_LIMIT, DEFAULT_RECENT_LIMIT, StorageManager};

impl StorageManager {
    /// 记录最近连接，并把同一主机的旧记录移动到队首。
    pub fn record_recent_connection(&mut self, connection: RecentConnection) {
        self.recent_connections
            .retain(|existing| existing.host_id != connection.host_id);
        self.recent_connections.insert(0, connection);
        self.recent_connections.truncate(DEFAULT_RECENT_LIMIT);
    }

    /// 追加命令历史，并优先裁剪已结束的旧记录，避免长期使用后存储无限增长。
    pub fn add_command_history(&mut self, item: CommandHistoryItem) {
        self.command_history.push(item);
        trim_command_history(&mut self.command_history, DEFAULT_COMMAND_HISTORY_LIMIT);
    }

    /// 按主机过滤命令历史。
    pub fn command_history_for_host(&self, host_id: HostId) -> Vec<&CommandHistoryItem> {
        self.command_history
            .iter()
            .filter(|item| item.host_id == Some(host_id))
            .collect()
    }
}

fn trim_command_history(history: &mut Vec<CommandHistoryItem>, limit: usize) {
    while history.len() > limit {
        let remove_index = history
            .iter()
            .position(|item| item.duration_ms.is_some())
            .unwrap_or(0);
        history.remove(remove_index);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smagical_core::CommandHistoryId;
    use uuid::Uuid;

    #[test]
    fn recent_connections_are_deduplicated_and_limited() {
        let mut storage = StorageManager::default();
        let repeated_host_id = HostId(Uuid::new_v4());

        storage.record_recent_connection(RecentConnection {
            host_id: repeated_host_id,
            label: "first".to_owned(),
            connected_at_unix_secs: 1,
        });
        storage.record_recent_connection(RecentConnection {
            host_id: repeated_host_id,
            label: "second".to_owned(),
            connected_at_unix_secs: 2,
        });

        for index in 0..25 {
            storage.record_recent_connection(RecentConnection {
                host_id: HostId(Uuid::new_v4()),
                label: format!("host-{index}"),
                connected_at_unix_secs: 100 + index,
            });
        }

        assert_eq!(storage.recent_count(), 20);
        assert_eq!(storage.recent_connections[0].label, "host-24");
        assert_eq!(
            storage
                .recent_connections
                .iter()
                .filter(|item| item.host_id == repeated_host_id)
                .count(),
            0
        );
    }

    #[test]
    fn command_history_can_be_filtered_by_host() {
        let mut storage = StorageManager::default();
        let host_id = HostId(Uuid::new_v4());
        let other_host_id = HostId(Uuid::new_v4());

        storage.add_command_history(CommandHistoryItem {
            id: CommandHistoryId(Uuid::new_v4()),
            host_id: Some(host_id),
            command: "pwd".to_owned(),
            working_directory: Some("/home/ops".to_owned()),
            exit_code: Some(0),
            started_at_unix_secs: 1,
            duration_ms: Some(3),
        });
        storage.add_command_history(CommandHistoryItem {
            id: CommandHistoryId(Uuid::new_v4()),
            host_id: Some(other_host_id),
            command: "whoami".to_owned(),
            working_directory: None,
            exit_code: Some(0),
            started_at_unix_secs: 2,
            duration_ms: Some(4),
        });

        let history = storage.command_history_for_host(host_id);

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].command, "pwd");
    }

    #[test]
    fn command_history_is_limited_and_keeps_newest_entries() {
        let mut storage = StorageManager::default();
        let host_id = HostId(Uuid::new_v4());

        for index in 0..(DEFAULT_COMMAND_HISTORY_LIMIT + 5) {
            storage.add_command_history(CommandHistoryItem {
                id: CommandHistoryId(Uuid::new_v4()),
                host_id: Some(host_id),
                command: format!("cmd-{index}"),
                working_directory: None,
                exit_code: Some(0),
                started_at_unix_secs: index as u64,
                duration_ms: Some(1),
            });
        }

        assert_eq!(
            storage.command_history_count(),
            DEFAULT_COMMAND_HISTORY_LIMIT
        );
        assert_eq!(storage.command_history[0].command, "cmd-5");
        assert_eq!(
            storage
                .command_history
                .last()
                .map(|item| item.command.as_str()),
            Some("cmd-504")
        );
    }

    #[test]
    fn command_history_trimming_prefers_completed_old_entries() {
        let mut history = Vec::new();
        let host_id = HostId(Uuid::new_v4());

        history.push(CommandHistoryItem {
            id: CommandHistoryId(Uuid::new_v4()),
            host_id: Some(host_id),
            command: "running".to_owned(),
            working_directory: None,
            exit_code: None,
            started_at_unix_secs: 1,
            duration_ms: None,
        });
        history.push(CommandHistoryItem {
            id: CommandHistoryId(Uuid::new_v4()),
            host_id: Some(host_id),
            command: "finished".to_owned(),
            working_directory: None,
            exit_code: Some(0),
            started_at_unix_secs: 2,
            duration_ms: Some(3),
        });
        history.push(CommandHistoryItem {
            id: CommandHistoryId(Uuid::new_v4()),
            host_id: Some(host_id),
            command: "new".to_owned(),
            working_directory: None,
            exit_code: None,
            started_at_unix_secs: 3,
            duration_ms: None,
        });

        trim_command_history(&mut history, 2);

        assert_eq!(history.len(), 2);
        assert_eq!(history[0].command, "running");
        assert_eq!(history[1].command, "new");
    }
}
