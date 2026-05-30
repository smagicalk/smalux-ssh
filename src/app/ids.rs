//! Slint 字符串标识到领域 ID 的转换。

use slint::SharedString;
use uuid::Uuid;

use crate::model::{CommandHistoryId, GroupId, HostId, SessionId};

/// 解析 Slint 传回的主机 ID。
pub(super) fn parse_host_id(id: &SharedString) -> Option<HostId> {
    parse_uuid(id).map(HostId)
}

/// 解析 Slint 传回的分组 ID，空字符串表示未分组。
pub(super) fn parse_optional_group_id(id: &SharedString) -> Option<Option<GroupId>> {
    if id.is_empty() {
        return Some(None);
    }

    parse_uuid(id).map(GroupId).map(Some)
}

/// 解析 Slint 传回的会话 ID。
pub(super) fn parse_session_id(id: &SharedString) -> Option<SessionId> {
    parse_uuid(id).map(SessionId)
}

/// 解析 Slint 传回的命令历史 ID。
pub(super) fn parse_command_history_id(id: &SharedString) -> Option<CommandHistoryId> {
    parse_uuid(id).map(CommandHistoryId)
}

fn parse_uuid(id: &SharedString) -> Option<Uuid> {
    Uuid::parse_str(id.as_str()).ok()
}
