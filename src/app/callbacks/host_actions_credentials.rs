//! 凭据页面回调。
//!
//! 这里集中处理私钥、密码、证书的创建、导入、导出、复制、删除和树节点移动回调。
//! 文件选择器和 Slint 字符串 ID 都停留在这一层，核心只接收明确的 `Message`。

use std::rc::Rc;

use slint::ComponentHandle;

use crate::model::Message;

use super::host_actions_helpers::{
    credential_group_kind_by_id, credential_kind_by_name, credential_secret_text,
    parse_credential_drop_target, parse_credential_group_row_id, parse_credential_kind,
    parse_credential_row_id, parse_key_algorithm, parse_optional_credential_group_row_id,
};
use super::{AppWindow, SharedAppState, apply_and_sync, apply_and_sync_success};

pub(super) fn bind(window: &AppWindow, state: SharedAppState) {
    window.on_choose_security_credential_file_path(|current_path| {
        crate::app::file_dialog::choose_security_credential_file_path(current_path.as_str()).into()
    });
    window.on_choose_security_credential_export_path(|current_path, kind_key| {
        crate::app::file_dialog::choose_security_credential_export_path(
            current_path.as_str(),
            kind_key.as_str(),
        )
        .into()
    });

    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_create_credential_group(move |name, parent_id, kind| {
            let Some(kind) = parse_credential_kind(&kind) else {
                return false;
            };
            apply_and_sync_success(
                &weak,
                &state,
                Message::CreateCredentialGroup {
                    name: name.to_string(),
                    kind,
                    parent_id: parse_optional_credential_group_row_id(&parent_id),
                },
            )
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_rename_credential_group(move |row_id, name| {
            let Some(group_id) = parse_credential_group_row_id(&row_id) else {
                return false;
            };
            apply_and_sync_success(
                &weak,
                &state,
                Message::RenameCredentialGroup {
                    group_id,
                    name: name.to_string(),
                },
            )
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_remove_credential_group(move |row_id| {
            let Some(group_id) = parse_credential_group_row_id(&row_id) else {
                return;
            };
            apply_and_sync(&weak, &state, Message::RemoveCredentialGroup { group_id });
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_duplicate_credential(move |row_id| {
            let Some(name) = parse_credential_row_id(&row_id) else {
                return;
            };
            apply_and_sync(
                &weak,
                &state,
                Message::DuplicateCredential {
                    name: name.to_owned(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_remove_credential(move |row_id| {
            let Some(name) = parse_credential_row_id(&row_id) else {
                return;
            };
            apply_and_sync(
                &weak,
                &state,
                Message::RemoveCredential {
                    name: name.to_owned(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_move_credential_tree_node(move |source_id, target_id| {
            let source_id = source_id.to_string();
            let target_id = target_id.to_string();
            let borrowed = state.borrow();
            let Some(target) = parse_credential_drop_target(&borrowed.core, &target_id) else {
                return false;
            };
            let message = if let Some(name) = parse_credential_row_id(&source_id) {
                let Some(kind) = credential_kind_by_name(&borrowed.core, name) else {
                    return false;
                };
                if target
                    .kind
                    .as_ref()
                    .is_some_and(|target_kind| target_kind != &kind)
                {
                    return false;
                }
                Message::MoveCredential {
                    name: name.to_owned(),
                    group_id: target.group_id,
                }
            } else if let Some(group_id) = parse_credential_group_row_id(&source_id) {
                let Some(kind) = credential_group_kind_by_id(&borrowed.core, group_id) else {
                    return false;
                };
                if target
                    .kind
                    .as_ref()
                    .is_some_and(|target_kind| target_kind != &kind)
                {
                    return false;
                }
                Message::MoveCredentialGroup {
                    group_id,
                    parent_id: target.group_id,
                }
            } else {
                return false;
            };
            drop(borrowed);
            apply_and_sync_success(&weak, &state, message)
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_create_credential_metadata(move |kind, name, group_id, secret_ref, algorithm| {
            let Some(kind) = parse_credential_kind(&kind) else {
                return false;
            };
            apply_and_sync_success(
                &weak,
                &state,
                Message::CreateCredentialMetadata {
                    kind,
                    name: name.to_string(),
                    group_id: parse_optional_credential_group_row_id(&group_id),
                    secret_ref: secret_ref.to_string(),
                    algorithm: parse_key_algorithm(&algorithm),
                },
            )
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_update_credential_metadata(move |credential_id, name, group_id, algorithm| {
            let Some(original_name) = parse_credential_row_id(&credential_id) else {
                return false;
            };
            apply_and_sync_success(
                &weak,
                &state,
                Message::UpdateCredentialMetadata {
                    original_name: original_name.to_owned(),
                    name: name.to_string(),
                    group_id: parse_optional_credential_group_row_id(&group_id),
                    algorithm: parse_key_algorithm(&algorithm),
                },
            )
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_update_credential_secret(move |credential_id, secret_text| {
            let Some(name) = parse_credential_row_id(&credential_id) else {
                return false;
            };
            apply_and_sync_success(
                &weak,
                &state,
                Message::UpdateCredentialSecret {
                    name: name.to_owned(),
                    secret_text: secret_text.to_string(),
                },
            )
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_generate_private_key_credential(move |name, group_id, algorithm| {
            apply_and_sync_success(
                &weak,
                &state,
                Message::GeneratePrivateKeyCredential {
                    name: name.to_string(),
                    group_id: parse_optional_credential_group_row_id(&group_id),
                    algorithm: parse_key_algorithm(&algorithm),
                },
            )
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_import_private_key_credential(move |name, group_id, source_path, algorithm| {
            apply_and_sync_success(
                &weak,
                &state,
                Message::ImportPrivateKeyCredential {
                    name: name.to_string(),
                    group_id: parse_optional_credential_group_row_id(&group_id),
                    source_path: source_path.to_string(),
                    algorithm: parse_key_algorithm(&algorithm),
                },
            )
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_save_password_credential(move |name, group_id, password| {
            apply_and_sync_success(
                &weak,
                &state,
                Message::SavePasswordCredential {
                    name: name.to_string(),
                    group_id: parse_optional_credential_group_row_id(&group_id),
                    password: password.to_string(),
                },
            )
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_import_private_key_text_credential(
            move |name, group_id, private_key_text, algorithm| {
                apply_and_sync_success(
                    &weak,
                    &state,
                    Message::ImportPrivateKeyTextCredential {
                        name: name.to_string(),
                        group_id: parse_optional_credential_group_row_id(&group_id),
                        private_key_text: private_key_text.to_string(),
                        algorithm: parse_key_algorithm(&algorithm),
                    },
                )
            },
        );
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_import_certificate_credential(move |name, group_id, source_path, algorithm| {
            apply_and_sync_success(
                &weak,
                &state,
                Message::ImportCertificateCredential {
                    name: name.to_string(),
                    group_id: parse_optional_credential_group_row_id(&group_id),
                    source_path: source_path.to_string(),
                    algorithm: parse_key_algorithm(&algorithm),
                },
            )
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_import_certificate_text_credential(
            move |name, group_id, certificate_text, algorithm| {
                apply_and_sync_success(
                    &weak,
                    &state,
                    Message::ImportCertificateTextCredential {
                        name: name.to_string(),
                        group_id: parse_optional_credential_group_row_id(&group_id),
                        certificate_text: certificate_text.to_string(),
                        algorithm: parse_key_algorithm(&algorithm),
                    },
                )
            },
        );
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_generate_certificate_credential(
            move |name,
                  group_id,
                  ca_private_key_ref,
                  subject_private_key_ref,
                  cert_type,
                  principals,
                  valid_days,
                  key_id,
                  serial| {
                apply_and_sync_success(
                    &weak,
                    &state,
                    Message::GenerateCertificateCredential {
                        name: name.to_string(),
                        group_id: parse_optional_credential_group_row_id(&group_id),
                        ca_private_key_ref: ca_private_key_ref.to_string(),
                        subject_private_key_ref: subject_private_key_ref.to_string(),
                        cert_type: cert_type.to_string(),
                        principals: principals.to_string(),
                        valid_days: valid_days.to_string(),
                        key_id: key_id.to_string(),
                        serial: serial.to_string(),
                    },
                )
            },
        );
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(&state);
        window.on_export_credential_secret(move |name, target_path| {
            let Some(name) = parse_credential_row_id(&name) else {
                return false;
            };
            apply_and_sync_success(
                &weak,
                &state,
                Message::ExportCredentialSecret {
                    name: name.to_owned(),
                    target_path: target_path.to_string(),
                },
            )
        });
    }
    {
        let state = Rc::clone(&state);
        window.on_read_credential_secret(move |credential_id| {
            credential_secret_text(&state.borrow().core, credential_id.as_str())
                .unwrap_or_default()
                .into()
        });
    }
}
