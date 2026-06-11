//! 主机和主机分组表单回调。
//!
//! 这个模块只负责把首页主机树、创建主机弹窗、创建分组弹窗传来的 UI 值转换成
//! `Message`。主机草稿校验、分组删除确认、复制主机等业务规则仍然集中在核心状态里。

use std::rc::Rc;

use slint::ComponentHandle;
use uuid::Uuid;

use crate::model::{ForwardId, JumpChainId, Message, ProxyId};

use super::host_actions_helpers::{
    parse_quick_host_auth_field, parse_quick_host_auth_kind, parse_quick_host_field,
};
use super::{AppWindow, SharedAppState, apply_and_sync, parse_host_id, parse_optional_group_id};

pub(super) fn bind(window: &AppWindow, state: SharedAppState) {
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_open_create_host_dialog(move || {
            apply_and_sync(&weak, &state, Message::OpenCreateHostDialog);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_open_create_host_dialog_in_group(move |group_id| {
            let Some(group_id) = parse_optional_group_id(&group_id) else {
                return;
            };
            apply_and_sync(
                &weak,
                &state,
                Message::OpenCreateHostDialogInGroup { group_id },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_open_create_group_parent_dialog(move |parent_id| {
            let Some(parent_id) = parse_optional_group_id(&parent_id) else {
                return;
            };
            apply_and_sync(
                &weak,
                &state,
                Message::OpenCreateGroupParentDialog { parent_id },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_select_create_group_parent(move |group_id| {
            let Some(parent_id) = parse_optional_group_id(&group_id) else {
                return;
            };
            apply_and_sync(
                &weak,
                &state,
                Message::SelectCreateGroupParent { parent_id },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_close_create_group_parent_dialog(move || {
            apply_and_sync(&weak, &state, Message::CloseCreateGroupParentDialog);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_confirm_create_group_parent(move || {
            apply_and_sync(&weak, &state, Message::ConfirmCreateGroupParent);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_open_create_group_dialog(move |parent_id| {
            let Some(parent_id) = parse_optional_group_id(&parent_id) else {
                return;
            };
            apply_and_sync(&weak, &state, Message::OpenCreateGroupDialog { parent_id });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_update_quick_group_name(move |name| {
            apply_and_sync(
                &weak,
                &state,
                Message::UpdateQuickGroupName {
                    name: name.to_string(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_select_quick_group_parent(move |group_id| {
            let Some(parent_id) = parse_optional_group_id(&group_id) else {
                return;
            };
            apply_and_sync(&weak, &state, Message::SelectQuickGroupParent { parent_id });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_close_create_group_dialog(move || {
            apply_and_sync(&weak, &state, Message::CloseCreateGroupDialog);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_save_quick_group(move || {
            apply_and_sync(&weak, &state, Message::SaveQuickGroup);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_open_edit_host_dialog(move |host_id| {
            let Some(host_id) = parse_host_id(&host_id) else {
                return;
            };
            apply_and_sync(&weak, &state, Message::OpenEditHostDialog { host_id });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_duplicate_host(move |host_id| {
            let Some(host_id) = parse_host_id(&host_id) else {
                return;
            };
            apply_and_sync(&weak, &state, Message::DuplicateHost { host_id });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_remove_host(move |host_id| {
            let Some(host_id) = parse_host_id(&host_id) else {
                return;
            };
            apply_and_sync(&weak, &state, Message::RequestRemoveHost { host_id });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_cancel_remove_host(move || {
            apply_and_sync(&weak, &state, Message::CancelRemoveHost);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_confirm_remove_host(move || {
            apply_and_sync(&weak, &state, Message::ConfirmRemoveHost);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_remove_group(move |group_id| {
            let Some(group_id) = parse_optional_group_id(&group_id).flatten() else {
                return;
            };
            apply_and_sync(&weak, &state, Message::RequestRemoveGroup { group_id });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_cancel_remove_group(move || {
            apply_and_sync(&weak, &state, Message::CancelRemoveGroup);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_confirm_remove_group(move || {
            apply_and_sync(&weak, &state, Message::ConfirmRemoveGroup);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_close_create_host_dialog(move || {
            apply_and_sync(&weak, &state, Message::CloseCreateHostDialog);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_update_quick_host_field(move |field, value| {
            let Some(field) = parse_quick_host_field(&field) else {
                return;
            };
            apply_and_sync(
                &weak,
                &state,
                Message::UpdateQuickHostDraft {
                    field,
                    value: value.to_string(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_select_quick_host_group(move |group_id| {
            let Some(group_id) = parse_optional_group_id(&group_id) else {
                return;
            };
            apply_and_sync(&weak, &state, Message::SelectQuickHostGroup { group_id });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_update_quick_host_auth_kind(move |kind| {
            let Some(kind) = parse_quick_host_auth_kind(&kind) else {
                return;
            };
            apply_and_sync(&weak, &state, Message::UpdateQuickHostAuthKind { kind });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_update_quick_host_auth_field(move |field, value| {
            let Some(field) = parse_quick_host_auth_field(&field) else {
                return;
            };
            apply_and_sync(
                &weak,
                &state,
                Message::UpdateQuickHostAuthField {
                    field,
                    value: value.to_string(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_toggle_quick_host_network_resource(move |kind, resource_id| {
            let Some(message) = parse_quick_host_network_toggle(&kind, &resource_id) else {
                return;
            };
            apply_and_sync(&weak, &state, message);
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_save_quick_host(move || {
            apply_and_sync(&weak, &state, Message::SaveQuickHost);
        });
    }
}

fn parse_quick_host_network_toggle(kind: &str, resource_id: &str) -> Option<Message> {
    let id = Uuid::parse_str(resource_id).ok()?;
    match kind {
        "ProxyAsset" => Some(Message::ToggleQuickHostNetworkProxy {
            proxy_id: ProxyId(id),
        }),
        "JumpChainAsset" => Some(Message::ToggleQuickHostNetworkJumpChain {
            chain_id: JumpChainId(id),
        }),
        "ForwardAsset" => Some(Message::ToggleQuickHostNetworkForward {
            forward_id: ForwardId(id),
        }),
        _ => None,
    }
}
