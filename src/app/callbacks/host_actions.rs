//! 主机动作回调。
//!
//! 这一层只把 Slint 传来的字符串 ID 和表单 key 转换成核心 `Message`。创建、编辑、复制、
//! 删除、分组选择等业务规则都在 `model/app_state` 内处理，方便以后替换 UI。

use std::rc::Rc;

use slint::ComponentHandle;

use crate::model::{
    Message, QuickHostAuthField, QuickHostAuthKind, QuickHostDraftField, ToolPanelMode,
};

use super::{
    AppWindow, SharedAppState, apply_and_sync, apply_and_sync_without_drain,
    apply_messages_and_sync, parse_host_id, parse_optional_group_id,
};

pub(super) fn bind(window: &AppWindow, state: SharedAppState) {
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_open_shell(move |host_id| {
            // Slint 不认识 Rust 的 HostId，只传稳定字符串；解析失败说明 UI 状态已过期。
            let Some(host_id) = parse_host_id(&host_id) else {
                return;
            };
            // 连接 shell 后端可能较慢，这里跳过同步 drain，让 worker 先接管命令。
            apply_and_sync_without_drain(&weak, &state, Message::OpenShell { host_id });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_open_host_sftp(move |host_id| {
            let Some(host_id) = parse_host_id(&host_id) else {
                return;
            };
            // 打开 SFTP 同时切换右侧工具面板，避免 UI 侧做多个状态写入。
            apply_messages_and_sync(
                &weak,
                &state,
                [
                    Message::OpenSftp {
                        host_id,
                        initial_dir: "/".to_owned(),
                    },
                    Message::OpenToolPanel {
                        mode: ToolPanelMode::Sftp,
                    },
                ],
            );
        });
    }
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
        window.on_save_quick_host(move || {
            apply_and_sync(&weak, &state, Message::SaveQuickHost);
        });
    }
}

fn parse_quick_host_field(field: &str) -> Option<QuickHostDraftField> {
    // 这些 key 来自 Slint 表单绑定，必须是稳定协议值，不允许使用本地化文案。
    match field {
        "Name" => Some(QuickHostDraftField::Name),
        "Address" => Some(QuickHostDraftField::Address),
        "Port" => Some(QuickHostDraftField::Port),
        "Username" => Some(QuickHostDraftField::Username),
        "Tags" => Some(QuickHostDraftField::Tags),
        "IconKey" => Some(QuickHostDraftField::IconKey),
        _ => None,
    }
}

fn parse_quick_host_auth_kind(kind: &str) -> Option<QuickHostAuthKind> {
    // 认证方式也使用稳定 key，展示文案由 view_model/i18n 决定。
    match kind {
        "Password" => Some(QuickHostAuthKind::Password),
        "Key" => Some(QuickHostAuthKind::Key),
        "ssh-agent" => Some(QuickHostAuthKind::Agent),
        "Certificate" => Some(QuickHostAuthKind::Certificate),
        _ => None,
    }
}

fn parse_quick_host_auth_field(field: &str) -> Option<QuickHostAuthField> {
    // 认证字段细节只在核心草稿里建模，UI 不直接构造 AuthProfile。
    match field {
        "AgentSource" => Some(QuickHostAuthField::AgentSource),
        "AgentCustomPipe" => Some(QuickHostAuthField::AgentCustomPipe),
        "PasswordSecretRef" => Some(QuickHostAuthField::PasswordSecretRef),
        "PrivateKeyRef" => Some(QuickHostAuthField::PrivateKeyRef),
        "PassphraseRef" => Some(QuickHostAuthField::PassphraseRef),
        "KeyHint" => Some(QuickHostAuthField::KeyHint),
        "CertificateRef" => Some(QuickHostAuthField::CertificateRef),
        _ => None,
    }
}
