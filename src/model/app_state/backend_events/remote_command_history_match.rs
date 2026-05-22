//! 远程命令历史匹配键提取。

use crate::model::{CommandHistoryId, HostId, SessionKind, SessionTab};

pub(super) struct RemoteCommandHistoryMatch {
    pub(super) host_id: HostId,
    pub(super) command: String,
    pub(super) history_id: Option<CommandHistoryId>,
}

pub(super) fn remote_command_history_match(tab: &SessionTab) -> Option<RemoteCommandHistoryMatch> {
    let SessionKind::RemoteCommand {
        command,
        history_id,
    } = &tab.kind
    else {
        return None;
    };

    Some(RemoteCommandHistoryMatch {
        host_id: tab.host_id?,
        command: command.clone(),
        history_id: *history_id,
    })
}
