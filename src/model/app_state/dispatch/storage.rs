//! 本地存储管理消息路由。
//!
//! 这里处理已经保存的数据的确认删除和安全资产维护。真正的落盘由 UI Adapter
//! 在状态变化后统一调用 `persist_storage`，领域函数只负责修改内存快照。

use super::super::{AppState, AppUpdateOutcome, Message};

impl AppState {
    /// 分发存储管理消息。
    ///
    /// 删除主机和分组采用 request/confirm 两步：request 只记录待确认目标，
    /// confirm 才真正修改存储，方便 UI 展示确认弹窗。
    pub(super) fn dispatch_storage_message(&mut self, message: Message) -> AppUpdateOutcome {
        match message {
            Message::ConfirmRemoveHost => self.confirm_remove_host(),
            Message::ConfirmRemoveGroup => self.confirm_remove_group(),
            Message::CreateCredentialGroup {
                name,
                kind,
                parent_id,
            } => self.create_credential_group(name, kind, parent_id),
            Message::RenameCredentialGroup { group_id, name } => {
                self.rename_credential_group(group_id, name)
            }
            Message::RemoveCredentialGroup { group_id } => self.remove_credential_group(group_id),
            Message::CreateCredentialMetadata {
                kind,
                name,
                group_id,
                secret_ref,
                algorithm,
            } => self.create_credential_metadata(kind, name, group_id, secret_ref, algorithm),
            Message::UpdateCredentialMetadata {
                original_name,
                name,
                group_id,
                algorithm,
            } => self.update_credential_metadata(&original_name, name, group_id, algorithm),
            Message::UpdateCredentialSecret { name, secret_text } => {
                self.update_credential_secret(&name, secret_text)
            }
            Message::GeneratePrivateKeyCredential {
                name,
                group_id,
                algorithm,
            } => self.generate_private_key_credential(name, group_id, algorithm),
            Message::SavePasswordCredential {
                name,
                group_id,
                password,
            } => self.save_password_credential(name, group_id, password),
            Message::ImportPrivateKeyCredential {
                name,
                group_id,
                source_path,
                algorithm,
            } => self.import_private_key_credential(name, group_id, source_path, algorithm),
            Message::ImportPrivateKeyTextCredential {
                name,
                group_id,
                private_key_text,
                algorithm,
            } => {
                self.import_private_key_text_credential(name, group_id, private_key_text, algorithm)
            }
            Message::ImportCertificateCredential {
                name,
                group_id,
                source_path,
                algorithm,
            } => self.import_certificate_credential(name, group_id, source_path, algorithm),
            Message::ImportCertificateTextCredential {
                name,
                group_id,
                certificate_text,
                algorithm,
            } => {
                self.import_certificate_text_credential(name, group_id, certificate_text, algorithm)
            }
            Message::GenerateCertificateCredential {
                name,
                group_id,
                ca_private_key_ref,
                subject_private_key_ref,
                cert_type,
                principals,
                valid_days,
                key_id,
                serial,
            } => self.generate_certificate_credential(
                name,
                group_id,
                ca_private_key_ref,
                subject_private_key_ref,
                cert_type,
                principals,
                valid_days,
                key_id,
                serial,
            ),
            Message::ExportCredentialSecret { name, target_path } => {
                self.export_credential_secret(&name, &target_path)
            }
            Message::DuplicateCredential { name } => self.duplicate_credential(&name),
            Message::RemoveCredential { name } => self.remove_credential(&name),
            Message::MoveCredential { name, group_id } => self.move_credential(&name, group_id),
            Message::MoveCredentialGroup {
                group_id,
                parent_id,
            } => self.move_credential_group(group_id, parent_id),
            Message::TrustKnownHost { host, port } => self.trust_known_host(&host, port),
            Message::RemoveKnownHost { host, port } => self.remove_known_host(&host, port),
            _ => unreachable!("非存储管理消息不应进入存储管理路由"),
        }
    }
}
