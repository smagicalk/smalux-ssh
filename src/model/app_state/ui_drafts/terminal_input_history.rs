//! 终端输入命令历史记录。
//!
//! 交互式终端输入也进入命令历史，方便命令面板复用。这里只记录用户提交的文本和关联主机，
//! 退出码、耗时等执行结果后续可由更完整的 shell 事件补齐。

use uuid::Uuid;

use crate::model::{CommandHistoryId, CommandHistoryItem, HostId};
use crate::storage::StorageManager;

use super::super::super::launch::unix_now_secs;

pub(super) fn record_terminal_input_history(
    storage: &mut StorageManager,
    host_id: Option<HostId>,
    command: String,
) {
    // 本地终端没有 host_id；远程 shell 会带上主机 ID，便于历史按主机过滤。
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
