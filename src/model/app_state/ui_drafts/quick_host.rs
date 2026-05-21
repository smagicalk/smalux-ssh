//! 快速新增主机草稿处理。

use uuid::Uuid;

use crate::model::{HostId, QuickHostAuthField, QuickHostAuthKind, QuickHostDraftField};

use super::super::{AppState, AppUpdateOutcome};
use super::draft_changed;

impl AppState {
    /// 更新快速新增主机表单草稿。
    pub(in crate::model::app_state) fn update_quick_host_draft(
        &mut self,
        field: QuickHostDraftField,
        value: String,
    ) -> AppUpdateOutcome {
        self.ui.set_quick_host_field(field, value);
        draft_changed()
    }

    /// 更新快速新增主机认证方式。
    pub(in crate::model::app_state) fn update_quick_host_auth_kind(
        &mut self,
        kind: QuickHostAuthKind,
    ) -> AppUpdateOutcome {
        self.ui.set_quick_host_auth_kind(kind);
        draft_changed()
    }

    /// 更新快速新增主机认证字段。
    pub(in crate::model::app_state) fn update_quick_host_auth_field(
        &mut self,
        field: QuickHostAuthField,
        value: String,
    ) -> AppUpdateOutcome {
        self.ui.set_quick_host_auth_field(field, value);
        draft_changed()
    }

    /// 保存快速新增主机。
    pub(in crate::model::app_state) fn save_quick_host(&mut self) -> AppUpdateOutcome {
        let host_id = HostId(Uuid::new_v4());
        let host = match self.ui.quick_host.build_host(host_id) {
            Ok(host) => host,
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(format!("主机表单无效：{error}")),
                    ..AppUpdateOutcome::default()
                };
            }
        };

        self.storage.upsert_host(host);
        self.ui.reset_quick_host();
        draft_changed()
    }
}
