//! 本地安全资产的管理操作。
//!
//! 这里专门处理凭据元数据和 Known Hosts 的增删改，避免把存储管理逻辑继续塞进
//! `app_state.rs` 主文件。
//!
//! 这个模块仍然属于核心状态层：它只操作 `StorageManager` 和 `UiState` 中的确认状态，
//! 不调用任何 Slint 组件。UI 需要弹窗或右键菜单时，只发送请求/确认/取消消息。

#[path = "storage_admin/credential.rs"]
mod credential;
#[path = "storage_admin/known_hosts.rs"]
mod known_hosts;
#[cfg(test)]
#[path = "storage_admin/tests.rs"]
mod tests;

use crate::model::{GroupId, HostId};

use super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 请求删除已保存主机，先记录待确认目标，不立即改动存储。
    pub(in crate::model::app_state) fn request_remove_host(
        &mut self,
        host_id: HostId,
    ) -> AppUpdateOutcome {
        // 删除是破坏性操作，第一步只写入 pending id，让 UI 有机会弹确认框。
        if self.storage.hosts.iter().any(|host| host.id == host_id) {
            self.ui.workspace.pending_delete_host_id = Some(host_id);
            return AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            };
        }

        AppUpdateOutcome {
            error: Some("主机不存在，无法删除".to_owned()),
            ..AppUpdateOutcome::default()
        }
    }

    /// 取消待确认的主机删除操作。
    pub(in crate::model::app_state) fn cancel_remove_host(&mut self) -> AppUpdateOutcome {
        let had_pending = self.ui.workspace.pending_delete_host_id.take().is_some();
        AppUpdateOutcome {
            state_changed: had_pending,
            ..AppUpdateOutcome::default()
        }
    }

    /// 确认删除当前待确认主机。
    pub(in crate::model::app_state) fn confirm_remove_host(&mut self) -> AppUpdateOutcome {
        let Some(host_id) = self.ui.workspace.pending_delete_host_id.take() else {
            return AppUpdateOutcome {
                error: Some("没有待删除的主机".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        self.remove_host_by_id(host_id)
    }

    /// 请求删除分组，先记录待确认目标，不立即改动存储。
    pub(in crate::model::app_state) fn request_remove_group(
        &mut self,
        group_id: GroupId,
    ) -> AppUpdateOutcome {
        // 分组可能包含子分组和主机，仍采用 pending + confirm 的两阶段流程。
        if self.storage.groups.iter().any(|group| group.id == group_id) {
            self.ui.workspace.pending_delete_group_id = Some(group_id);
            return AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            };
        }

        AppUpdateOutcome {
            error: Some("分组不存在，无法删除".to_owned()),
            ..AppUpdateOutcome::default()
        }
    }

    /// 取消待确认的分组删除操作。
    pub(in crate::model::app_state) fn cancel_remove_group(&mut self) -> AppUpdateOutcome {
        let had_pending = self.ui.workspace.pending_delete_group_id.take().is_some();
        AppUpdateOutcome {
            state_changed: had_pending,
            ..AppUpdateOutcome::default()
        }
    }

    /// 确认删除当前待确认分组。
    pub(in crate::model::app_state) fn confirm_remove_group(&mut self) -> AppUpdateOutcome {
        let Some(group_id) = self.ui.workspace.pending_delete_group_id.take() else {
            return AppUpdateOutcome {
                error: Some("没有待删除的分组".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        // StorageManager 负责递归删除的具体策略，状态层只关心结果和错误提示。
        if self.storage.remove_group_recursive(group_id) {
            return AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            };
        }

        AppUpdateOutcome {
            error: Some("分组不存在，无法删除".to_owned()),
            ..AppUpdateOutcome::default()
        }
    }

    /// 删除已保存主机，并清理主机相关的本地索引。
    fn remove_host_by_id(&mut self, host_id: HostId) -> AppUpdateOutcome {
        // 当前只从主机集合移除；后续如果 SQLite 层增加外键/软删除，可以保持调用点不变。
        if self.storage.remove_host(host_id) {
            return AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            };
        }

        AppUpdateOutcome {
            error: Some("主机不存在，无法删除".to_owned()),
            ..AppUpdateOutcome::default()
        }
    }
}
