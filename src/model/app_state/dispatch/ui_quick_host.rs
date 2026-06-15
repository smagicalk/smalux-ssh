//! 主机创建、编辑和删除确认相关 UI 消息。

use super::super::super::{AppState, AppUpdateOutcome, Message};

impl AppState {
    pub(super) fn try_dispatch_quick_host_ui_message(
        &mut self,
        message: &Message,
    ) -> Option<AppUpdateOutcome> {
        Some(match message {
            Message::UpdateQuickHostDraft { field, value } => {
                self.ui.set_quick_host_field(field.clone(), value.clone());
                draft_changed()
            }
            Message::SelectQuickHostGroup { group_id } => {
                self.ui.select_quick_host_group(*group_id);
                draft_changed()
            }
            Message::UpdateQuickHostAuthKind { kind } => {
                self.ui.set_quick_host_auth_kind(kind.clone());
                draft_changed()
            }
            Message::UpdateQuickHostAuthField { field, value } => {
                self.ui
                    .set_quick_host_auth_field(field.clone(), value.clone());
                draft_changed()
            }
            Message::ToggleQuickHostNetworkProxy { proxy_id } => {
                if !self
                    .storage
                    .proxy_assets
                    .iter()
                    .any(|asset| asset.id == *proxy_id)
                {
                    return Some(AppUpdateOutcome {
                        error: Some("代理资源不存在，无法选择".to_owned()),
                        ..AppUpdateOutcome::default()
                    });
                }
                self.ui.toggle_quick_host_proxy(*proxy_id);
                draft_changed()
            }
            Message::ToggleQuickHostNetworkJumpChain { chain_id } => {
                if !self
                    .storage
                    .jump_chain_assets
                    .iter()
                    .any(|asset| asset.id == *chain_id)
                {
                    return Some(AppUpdateOutcome {
                        error: Some("跳板资源不存在，无法选择".to_owned()),
                        ..AppUpdateOutcome::default()
                    });
                }
                self.ui.toggle_quick_host_jump_chain(*chain_id);
                draft_changed()
            }
            Message::ToggleQuickHostNetworkForward { forward_id } => {
                if !self
                    .storage
                    .forward_assets
                    .iter()
                    .any(|asset| asset.id == *forward_id)
                {
                    return Some(AppUpdateOutcome {
                        error: Some("转发资源不存在，无法选择".to_owned()),
                        ..AppUpdateOutcome::default()
                    });
                }
                self.ui.toggle_quick_host_forward(*forward_id);
                draft_changed()
            }
            Message::SaveQuickHost => {
                let editing_host_id = self.ui.quick_host.editing_host_id;
                let existing_host = editing_host_id.and_then(|host_id| {
                    self.storage
                        .hosts
                        .iter()
                        .find(|host| host.id == host_id)
                        .cloned()
                });
                if editing_host_id.is_some() && existing_host.is_none() {
                    return Some(AppUpdateOutcome {
                        error: Some("主机不存在，无法保存编辑".to_owned()),
                        ..AppUpdateOutcome::default()
                    });
                }
                let host_id =
                    editing_host_id.unwrap_or_else(|| crate::model::HostId(uuid::Uuid::new_v4()));
                let host = match self
                    .ui
                    .quick_host
                    .build_host_with_existing(host_id, existing_host.as_ref())
                {
                    Ok(host) => host,
                    Err(error) => {
                        return Some(AppUpdateOutcome {
                            error: Some(format!("主机表单无效：{error}")),
                            ..AppUpdateOutcome::default()
                        });
                    }
                };
                let outcome = self.core.save_host_record(host, editing_host_id);
                if outcome.error.is_some() {
                    return Some(outcome);
                }
                self.ui.reset_quick_host();
                self.ui.workspace.create_host_dialog_open = false;
                draft_changed()
            }
            Message::OpenCreateHostDialogInGroup { group_id } => {
                if group_id
                    .is_some_and(|id| !self.storage.groups.iter().any(|group| group.id == id))
                {
                    return Some(AppUpdateOutcome {
                        error: Some("分组不存在，无法创建主机".to_owned()),
                        ..AppUpdateOutcome::default()
                    });
                }
                self.ui.reset_quick_host();
                self.ui.quick_host.group_id = *group_id;
                self.ui.workspace.create_host_dialog_open = true;
                self.ui.workspace.create_group_dialog_open = false;
                self.ui.workspace.create_group_parent_dialog_open = false;
                self.ui.workspace.pending_create_group_parent_id = None;
                draft_changed()
            }
            Message::OpenCreateGroupParentDialog { parent_id } => {
                if parent_id
                    .is_some_and(|id| !self.storage.groups.iter().any(|group| group.id == id))
                {
                    return Some(AppUpdateOutcome {
                        error: Some("父级分组不存在，无法选择".to_owned()),
                        ..AppUpdateOutcome::default()
                    });
                }
                self.ui.workspace.pending_create_group_parent_id = *parent_id;
                self.ui.workspace.create_group_parent_dialog_open = true;
                self.ui.workspace.create_group_dialog_open = false;
                self.ui.workspace.create_host_dialog_open = false;
                draft_changed()
            }
            Message::SelectCreateGroupParent { parent_id } => {
                if parent_id
                    .is_some_and(|id| !self.storage.groups.iter().any(|group| group.id == id))
                {
                    return Some(AppUpdateOutcome {
                        error: Some("父级分组不存在，无法选择".to_owned()),
                        ..AppUpdateOutcome::default()
                    });
                }
                self.ui.workspace.pending_create_group_parent_id = *parent_id;
                draft_changed()
            }
            Message::CloseCreateGroupParentDialog => {
                let had_open = self.ui.workspace.create_group_parent_dialog_open;
                let had_pending = self
                    .ui
                    .workspace
                    .pending_create_group_parent_id
                    .take()
                    .is_some();
                self.ui.workspace.create_group_parent_dialog_open = false;
                AppUpdateOutcome {
                    state_changed: had_open || had_pending,
                    ..AppUpdateOutcome::default()
                }
            }
            Message::ConfirmCreateGroupParent => {
                let parent_id = self.ui.workspace.pending_create_group_parent_id;
                if parent_id
                    .is_some_and(|id| !self.storage.groups.iter().any(|group| group.id == id))
                {
                    return Some(AppUpdateOutcome {
                        error: Some("父级分组不存在，无法创建子分组".to_owned()),
                        ..AppUpdateOutcome::default()
                    });
                }
                self.ui.quick_group = crate::model::QuickGroupDraft::with_parent(parent_id);
                self.ui.workspace.create_group_dialog_open = true;
                self.ui.workspace.create_group_parent_dialog_open = false;
                self.ui.workspace.pending_create_group_parent_id = None;
                self.ui.workspace.create_host_dialog_open = false;
                draft_changed()
            }
            Message::OpenCreateGroupDialog { parent_id } => {
                if parent_id
                    .is_some_and(|id| !self.storage.groups.iter().any(|group| group.id == id))
                {
                    return Some(AppUpdateOutcome {
                        error: Some("父级分组不存在，无法创建子分组".to_owned()),
                        ..AppUpdateOutcome::default()
                    });
                }
                self.ui.quick_group = crate::model::QuickGroupDraft::with_parent(*parent_id);
                self.ui.workspace.create_group_dialog_open = true;
                self.ui.workspace.create_group_parent_dialog_open = false;
                self.ui.workspace.pending_create_group_parent_id = None;
                self.ui.workspace.create_host_dialog_open = false;
                draft_changed()
            }
            Message::UpdateQuickGroupName { name } => {
                self.ui.quick_group.name = name.clone();
                draft_changed()
            }
            Message::SelectQuickGroupParent { parent_id } => {
                if parent_id
                    .is_some_and(|id| !self.storage.groups.iter().any(|group| group.id == id))
                {
                    return Some(AppUpdateOutcome {
                        error: Some("父级分组不存在，无法选择".to_owned()),
                        ..AppUpdateOutcome::default()
                    });
                }
                self.ui.quick_group.parent_id = *parent_id;
                draft_changed()
            }
            Message::CloseCreateGroupDialog => {
                let had_open = self.ui.workspace.create_group_dialog_open;
                let had_group_name = !self.ui.quick_group.name.is_empty();
                let had_group_parent = self.ui.quick_group.parent_id.is_some();
                self.ui.workspace.create_group_dialog_open = false;
                self.ui.quick_group = crate::model::QuickGroupDraft::default();
                AppUpdateOutcome {
                    state_changed: had_open || had_group_name || had_group_parent,
                    ..AppUpdateOutcome::default()
                }
            }
            Message::SaveQuickGroup => {
                let name = self.ui.quick_group.name.trim().to_owned();
                if name.is_empty() {
                    return Some(AppUpdateOutcome {
                        error: Some("分组名称不能为空".to_owned()),
                        ..AppUpdateOutcome::default()
                    });
                }
                let parent_id = self.ui.quick_group.parent_id;
                if parent_id
                    .is_some_and(|id| !self.storage.groups.iter().any(|group| group.id == id))
                {
                    return Some(AppUpdateOutcome {
                        error: Some("父级分组不存在，无法保存".to_owned()),
                        ..AppUpdateOutcome::default()
                    });
                }
                self.storage.upsert_group(crate::model::HostGroup {
                    id: crate::model::GroupId(uuid::Uuid::new_v4()),
                    name,
                    parent_id,
                });
                self.ui.workspace.create_group_dialog_open = false;
                self.ui.quick_group = crate::model::QuickGroupDraft::default();
                draft_changed()
            }
            Message::OpenCreateHostDialog => {
                self.ui.reset_quick_host();
                self.ui.workspace.create_host_dialog_open = true;
                self.ui.workspace.create_group_dialog_open = false;
                AppUpdateOutcome {
                    state_changed: true,
                    ..AppUpdateOutcome::default()
                }
            }
            Message::OpenEditHostDialog { host_id } => {
                let Some(host) = self
                    .storage
                    .hosts
                    .iter()
                    .find(|host| host.id == *host_id)
                    .cloned()
                else {
                    return Some(AppUpdateOutcome {
                        error: Some("主机不存在，无法编辑".to_owned()),
                        ..AppUpdateOutcome::default()
                    });
                };
                self.ui.edit_quick_host(&host);
                self.ui.workspace.create_host_dialog_open = true;
                draft_changed()
            }
            Message::DuplicateHost { host_id } => self.core.duplicate_host_record(*host_id),
            Message::CloseCreateHostDialog => AppUpdateOutcome {
                state_changed: std::mem::replace(
                    &mut self.ui.workspace.create_host_dialog_open,
                    false,
                ),
                ..AppUpdateOutcome::default()
            },
            Message::RequestRemoveHost { host_id } => {
                if self.storage.hosts.iter().any(|host| host.id == *host_id) {
                    self.ui.workspace.pending_delete_host_id = Some(*host_id);
                    AppUpdateOutcome {
                        state_changed: true,
                        ..AppUpdateOutcome::default()
                    }
                } else {
                    AppUpdateOutcome {
                        error: Some("主机不存在，无法删除".to_owned()),
                        ..AppUpdateOutcome::default()
                    }
                }
            }
            Message::CancelRemoveHost => AppUpdateOutcome {
                state_changed: self.ui.workspace.pending_delete_host_id.take().is_some(),
                ..AppUpdateOutcome::default()
            },
            Message::RequestRemoveGroup { group_id } => {
                if self
                    .storage
                    .groups
                    .iter()
                    .any(|group| group.id == *group_id)
                {
                    self.ui.workspace.pending_delete_group_id = Some(*group_id);
                    AppUpdateOutcome {
                        state_changed: true,
                        ..AppUpdateOutcome::default()
                    }
                } else {
                    AppUpdateOutcome {
                        error: Some("分组不存在，无法删除".to_owned()),
                        ..AppUpdateOutcome::default()
                    }
                }
            }
            Message::CancelRemoveGroup => AppUpdateOutcome {
                state_changed: self.ui.workspace.pending_delete_group_id.take().is_some(),
                ..AppUpdateOutcome::default()
            },
            _ => return None,
        })
    }
}

fn draft_changed() -> AppUpdateOutcome {
    AppUpdateOutcome {
        state_changed: true,
        ..AppUpdateOutcome::default()
    }
}
