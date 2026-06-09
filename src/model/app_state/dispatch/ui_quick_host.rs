//! 主机创建、编辑和删除确认相关 UI 消息。

use super::super::super::{AppState, AppUpdateOutcome, Message};

impl AppState {
    pub(super) fn try_dispatch_quick_host_ui_message(
        &mut self,
        message: &Message,
    ) -> Option<AppUpdateOutcome> {
        Some(match message {
            Message::UpdateQuickHostDraft { field, value } => {
                self.update_quick_host_draft(field.clone(), value.clone())
            }
            Message::SelectQuickHostGroup { group_id } => self.select_quick_host_group(*group_id),
            Message::UpdateQuickHostAuthKind { kind } => {
                self.update_quick_host_auth_kind(kind.clone())
            }
            Message::UpdateQuickHostAuthField { field, value } => {
                self.update_quick_host_auth_field(field.clone(), value.clone())
            }
            Message::SaveQuickHost => self.save_quick_host(),
            Message::OpenCreateHostDialogInGroup { group_id } => {
                self.open_create_host_dialog_in_group(*group_id)
            }
            Message::OpenCreateGroupParentDialog { parent_id } => {
                self.open_create_group_parent_dialog(*parent_id)
            }
            Message::SelectCreateGroupParent { parent_id } => {
                self.select_create_group_parent(*parent_id)
            }
            Message::CloseCreateGroupParentDialog => self.close_create_group_parent_dialog(),
            Message::ConfirmCreateGroupParent => self.confirm_create_group_parent(),
            Message::OpenCreateGroupDialog { parent_id } => {
                self.open_create_group_dialog(*parent_id)
            }
            Message::UpdateQuickGroupName { name } => self.update_quick_group_name(name.clone()),
            Message::SelectQuickGroupParent { parent_id } => {
                self.select_quick_group_parent(*parent_id)
            }
            Message::CloseCreateGroupDialog => self.close_create_group_dialog(),
            Message::SaveQuickGroup => self.save_quick_group(),
            Message::OpenCreateHostDialog => self.open_create_host_dialog(),
            Message::OpenEditHostDialog { host_id } => self.open_edit_host_dialog(*host_id),
            Message::DuplicateHost { host_id } => self.duplicate_host(*host_id),
            Message::CloseCreateHostDialog => self.close_create_host_dialog(),
            Message::RequestRemoveHost { host_id } => self.request_remove_host(*host_id),
            Message::CancelRemoveHost => self.cancel_remove_host(),
            Message::RequestRemoveGroup { group_id } => self.request_remove_group(*group_id),
            Message::CancelRemoveGroup => self.cancel_remove_group(),
            _ => return None,
        })
    }
}
