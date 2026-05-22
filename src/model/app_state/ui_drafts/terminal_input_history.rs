//! 终端输入命令历史记录。

use uuid::Uuid;

use crate::model::{CommandHistoryId, CommandHistoryItem, HostId};
use crate::storage::StorageManager;

use super::super::super::launch::unix_now_secs;

pub(super) fn record_terminal_input_history(
    storage: &mut StorageManager,
    host_id: Option<HostId>,
    command: String,
) {
    storage.add_command_history(CommandHistoryItem {
        id: CommandHistoryId(Uuid::new_v4()),
        host_id,
        command,
        working_directory: None,
        exit_code: None,
        started_at_unix_secs: unix_now_secs(),
        duration_ms: None,
    });
}
