//! 本地安全资产的管理操作。
//!
//! 这里专门处理凭据元数据和 Known Hosts 的增删改，避免把存储管理逻辑继续塞进
//! `app_state.rs` 主文件。
//!
//! 这个模块仍然属于核心状态层：核心动作只操作 `StorageManager`，旧 `AppState`
//! 包装继续承接过渡期的确认状态兼容。

#[path = "storage_admin/credential.rs"]
mod credential;
#[path = "storage_admin/credential_certificate_params.rs"]
mod credential_certificate_params;
#[path = "storage_admin/credential_groups.rs"]
mod credential_groups;
#[path = "storage_admin/credential_ids.rs"]
mod credential_ids;
#[path = "storage_admin/credential_material.rs"]
mod credential_material;
#[path = "storage_admin/credential_material_certificate.rs"]
mod credential_material_certificate;
#[path = "storage_admin/credential_material_generate.rs"]
mod credential_material_generate;
#[path = "storage_admin/credential_payload.rs"]
mod credential_payload;
#[path = "storage_admin/credential_refs.rs"]
mod credential_refs;
#[path = "storage_admin/known_hosts.rs"]
mod known_hosts;
#[path = "storage_admin/network_assets.rs"]
mod network_assets;
#[cfg(test)]
#[path = "storage_admin/tests.rs"]
mod tests;

use crate::model::{GroupId, HostId};

use crate::core::CoreState;

use super::AppUpdateOutcome;

impl CoreState {
    /// 删除已保存主机的稳定核心入口。
    #[cfg_attr(not(feature = "desktop"), allow(dead_code))]
    pub(crate) fn remove_host_record_action(&mut self, host_id: HostId) -> AppUpdateOutcome {
        self.remove_host_by_id(host_id)
    }

    /// 递归删除已保存主机分组的稳定核心入口。
    #[cfg_attr(not(feature = "desktop"), allow(dead_code))]
    pub(crate) fn remove_group_record_recursive_action(
        &mut self,
        group_id: GroupId,
    ) -> AppUpdateOutcome {
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
