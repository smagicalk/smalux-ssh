//! 快捷命令错误结果构造。

use crate::model::{HostId, SnippetGroupId, SnippetId};

use super::super::AppUpdateOutcome;

pub(super) fn missing_host(host_id: HostId) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!("找不到主机：{}", host_id.0)),
        ..AppUpdateOutcome::default()
    }
}

pub(super) fn missing_snippet(snippet_id: SnippetId) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!("找不到快捷命令：{}", snippet_id.0)),
        ..AppUpdateOutcome::default()
    }
}

pub(super) fn missing_snippet_group(group_id: SnippetGroupId) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!("找不到快捷命令分组：{}", group_id.0)),
        ..AppUpdateOutcome::default()
    }
}
