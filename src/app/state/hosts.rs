//! 主机与分组弹窗草稿提交逻辑。

use uuid::Uuid;

use crate::model::{AppUpdateOutcome, ForwardId, GroupId, HostGroup, HostId, JumpChainId, ProxyId};

use super::{DesktopAppState, draft_changed};

impl DesktopAppState {
    pub(super) fn request_remove_host_dialog(&mut self, host_id: HostId) -> AppUpdateOutcome {
        if self
            .core
            .storage
            .hosts
            .iter()
            .any(|host| host.id == host_id)
        {
            self.ui.workspace.pending_delete_host_id = Some(host_id);
            return draft_changed();
        }
        AppUpdateOutcome {
            error: Some("主机不存在，无法删除".to_owned()),
            ..AppUpdateOutcome::default()
        }
    }

    pub(super) fn toggle_quick_host_network_proxy_local(
        &mut self,
        proxy_id: ProxyId,
    ) -> AppUpdateOutcome {
        if !self
            .core
            .storage
            .proxy_assets
            .iter()
            .any(|asset| asset.id == proxy_id)
        {
            return AppUpdateOutcome {
                error: Some("代理资源不存在，无法选择".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.ui.toggle_quick_host_proxy(proxy_id);
        draft_changed()
    }

    pub(super) fn toggle_quick_host_network_jump_chain_local(
        &mut self,
        chain_id: JumpChainId,
    ) -> AppUpdateOutcome {
        if !self
            .core
            .storage
            .jump_chain_assets
            .iter()
            .any(|asset| asset.id == chain_id)
        {
            return AppUpdateOutcome {
                error: Some("跳板资源不存在，无法选择".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.ui.toggle_quick_host_jump_chain(chain_id);
        draft_changed()
    }

    pub(super) fn toggle_quick_host_network_forward_local(
        &mut self,
        forward_id: ForwardId,
    ) -> AppUpdateOutcome {
        if !self
            .core
            .storage
            .forward_assets
            .iter()
            .any(|asset| asset.id == forward_id)
        {
            return AppUpdateOutcome {
                error: Some("转发资源不存在，无法选择".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.ui.toggle_quick_host_forward(forward_id);
        draft_changed()
    }

    pub(super) fn save_quick_host_local(&mut self) -> AppUpdateOutcome {
        let editing_host_id = self.ui.quick_host.editing_host_id;
        let existing_host = editing_host_id.and_then(|host_id| {
            self.core
                .storage
                .hosts
                .iter()
                .find(|host| host.id == host_id)
                .cloned()
        });

        if editing_host_id.is_some() && existing_host.is_none() {
            return AppUpdateOutcome {
                error: Some("主机不存在，无法保存编辑".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let host_id = editing_host_id.unwrap_or_else(|| HostId(Uuid::new_v4()));
        let host = match self
            .ui
            .quick_host
            .build_host_with_existing(host_id, existing_host.as_ref())
        {
            Ok(host) => host,
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(format!("主机表单无效：{error}")),
                    ..AppUpdateOutcome::default()
                };
            }
        };

        let outcome = self.core.save_host_record(host, editing_host_id);
        if outcome.error.is_some() {
            return outcome;
        }
        self.ui.reset_quick_host();
        self.ui.workspace.create_host_dialog_open = false;
        draft_changed()
    }

    pub(super) fn save_quick_group_local(&mut self) -> AppUpdateOutcome {
        let name = self.ui.quick_group.name.trim().to_owned();
        if name.is_empty() {
            return AppUpdateOutcome {
                error: Some("分组名称不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let parent_id = self.ui.quick_group.parent_id;
        if parent_id.is_some_and(|id| !self.core.storage.groups.iter().any(|group| group.id == id))
        {
            return AppUpdateOutcome {
                error: Some("父级分组不存在，无法保存".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let outcome = self.core.save_group_record(HostGroup {
            id: GroupId(Uuid::new_v4()),
            name,
            parent_id,
        });
        if outcome.error.is_some() {
            return outcome;
        }
        self.ui.workspace.create_group_dialog_open = false;
        self.ui.quick_group = crate::model::QuickGroupDraft::default();
        draft_changed()
    }

    pub(super) fn cancel_remove_host_dialog(&mut self) -> AppUpdateOutcome {
        let had_pending = self.ui.workspace.pending_delete_host_id.take().is_some();
        AppUpdateOutcome {
            state_changed: had_pending,
            ..AppUpdateOutcome::default()
        }
    }

    pub(super) fn confirm_remove_host_dialog(&mut self) -> AppUpdateOutcome {
        let Some(host_id) = self.ui.workspace.pending_delete_host_id.take() else {
            return AppUpdateOutcome {
                error: Some("没有待删除的主机".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        self.core.remove_host_record_action(host_id)
    }

    pub(super) fn request_remove_group_dialog(&mut self, group_id: GroupId) -> AppUpdateOutcome {
        if self
            .core
            .storage
            .groups
            .iter()
            .any(|group| group.id == group_id)
        {
            self.ui.workspace.pending_delete_group_id = Some(group_id);
            return draft_changed();
        }
        AppUpdateOutcome {
            error: Some("分组不存在，无法删除".to_owned()),
            ..AppUpdateOutcome::default()
        }
    }

    pub(super) fn cancel_remove_group_dialog(&mut self) -> AppUpdateOutcome {
        let had_pending = self.ui.workspace.pending_delete_group_id.take().is_some();
        AppUpdateOutcome {
            state_changed: had_pending,
            ..AppUpdateOutcome::default()
        }
    }

    pub(super) fn confirm_remove_group_dialog(&mut self) -> AppUpdateOutcome {
        let Some(group_id) = self.ui.workspace.pending_delete_group_id.take() else {
            return AppUpdateOutcome {
                error: Some("没有待删除的分组".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        self.core.remove_group_record_recursive_action(group_id)
    }

    pub(super) fn open_create_host_dialog_local(&mut self) -> AppUpdateOutcome {
        self.ui.reset_quick_host();
        self.ui.workspace.create_host_dialog_open = true;
        self.ui.workspace.create_group_dialog_open = false;
        draft_changed()
    }

    pub(super) fn close_create_host_dialog_local(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.create_host_dialog_open = false;
        draft_changed()
    }

    pub(super) fn open_edit_host_dialog_local(&mut self, host_id: HostId) -> AppUpdateOutcome {
        let Some(host) = self
            .core
            .storage
            .hosts
            .iter()
            .find(|host| host.id == host_id)
            .cloned()
        else {
            return AppUpdateOutcome {
                error: Some("主机不存在，无法编辑".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        self.ui.edit_quick_host(&host);
        self.ui.workspace.create_host_dialog_open = true;
        draft_changed()
    }

    pub(super) fn open_create_host_dialog_in_group_local(
        &mut self,
        group_id: Option<GroupId>,
    ) -> AppUpdateOutcome {
        if group_id.is_some_and(|id| !self.core.storage.groups.iter().any(|group| group.id == id)) {
            return AppUpdateOutcome {
                error: Some("分组不存在，无法创建主机".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.ui.reset_quick_host();
        self.ui.quick_host.group_id = group_id;
        self.ui.workspace.create_host_dialog_open = true;
        self.ui.workspace.create_group_dialog_open = false;
        self.ui.workspace.create_group_parent_dialog_open = false;
        self.ui.workspace.pending_create_group_parent_id = None;
        draft_changed()
    }

    pub(super) fn open_create_group_parent_dialog_local(
        &mut self,
        parent_id: Option<GroupId>,
    ) -> AppUpdateOutcome {
        if parent_id.is_some_and(|id| !self.core.storage.groups.iter().any(|group| group.id == id))
        {
            return AppUpdateOutcome {
                error: Some("父级分组不存在，无法选择".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.ui.workspace.pending_create_group_parent_id = parent_id;
        self.ui.workspace.create_group_parent_dialog_open = true;
        self.ui.workspace.create_group_dialog_open = false;
        self.ui.workspace.create_host_dialog_open = false;
        draft_changed()
    }

    pub(super) fn select_create_group_parent_local(
        &mut self,
        parent_id: Option<GroupId>,
    ) -> AppUpdateOutcome {
        if parent_id.is_some_and(|id| !self.core.storage.groups.iter().any(|group| group.id == id))
        {
            return AppUpdateOutcome {
                error: Some("父级分组不存在，无法选择".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }
        self.ui.workspace.pending_create_group_parent_id = parent_id;
        draft_changed()
    }

    pub(super) fn close_create_group_parent_dialog_local(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.create_group_parent_dialog_open = false;
        self.ui.workspace.pending_create_group_parent_id = None;
        draft_changed()
    }

    pub(super) fn confirm_create_group_parent_local(&mut self) -> AppUpdateOutcome {
        let parent_id = self.ui.workspace.pending_create_group_parent_id;
        self.open_create_group_dialog_local(parent_id)
    }

    pub(super) fn open_create_group_dialog_local(
        &mut self,
        parent_id: Option<GroupId>,
    ) -> AppUpdateOutcome {
        if parent_id.is_some_and(|id| !self.core.storage.groups.iter().any(|group| group.id == id))
        {
            return AppUpdateOutcome {
                error: Some("父级分组不存在，无法创建子分组".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        self.ui.quick_group = crate::model::QuickGroupDraft::with_parent(parent_id);
        self.ui.workspace.create_group_dialog_open = true;
        self.ui.workspace.create_group_parent_dialog_open = false;
        self.ui.workspace.pending_create_group_parent_id = None;
        self.ui.workspace.create_host_dialog_open = false;
        draft_changed()
    }

    pub(super) fn select_quick_group_parent_local(
        &mut self,
        parent_id: Option<GroupId>,
    ) -> AppUpdateOutcome {
        if parent_id.is_some_and(|id| !self.core.storage.groups.iter().any(|group| group.id == id))
        {
            return AppUpdateOutcome {
                error: Some("父级分组不存在，无法选择".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }
        self.ui.quick_group.parent_id = parent_id;
        draft_changed()
    }

    pub(super) fn close_create_group_dialog_local(&mut self) -> AppUpdateOutcome {
        self.ui.workspace.create_group_dialog_open = false;
        self.ui.quick_group = crate::model::QuickGroupDraft::default();
        draft_changed()
    }
}
