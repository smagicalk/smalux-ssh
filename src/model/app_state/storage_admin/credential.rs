//! 凭据元数据管理。
//!
//! 这里管理的是“凭据索引/元数据”，不是私钥或密码本体。后续接入加密存储时，真正敏感
//! 数据应在 storage/security 层处理，状态层只通过名称或 ID 发起管理动作。

use crate::core::CoreState;
use crate::model::{
    CredentialGroupId, CredentialKind, CredentialMetadata, KeyAlgorithm, SecretMaterialKind,
    SecretRecord, SecretRef,
};
use russh::keys::PrivateKey;
use std::path::Path;

use super::super::AppUpdateOutcome;
use super::credential_groups::validate_credential_group;
use super::credential_ids::new_credential_id;
use super::credential_payload::{
    credential_payload_validation_error, decode_plaintext_private_key, inspect_credential_payload,
    local_secret_kind_for_credential, replacement_payload,
};
use super::credential_refs::{
    auth_profile_uses_secret_ref, credential_secret_namespace, next_credential_copy_name,
    next_secret_ref,
};

impl CoreState {
    /// 创建或更新凭据元数据。
    pub(crate) fn create_credential_metadata(
        &mut self,
        kind: CredentialKind,
        name: String,
        group_id: Option<CredentialGroupId>,
        secret_ref: String,
        algorithm: Option<KeyAlgorithm>,
    ) -> AppUpdateOutcome {
        let name = name.trim();
        let secret_ref = secret_ref.trim();

        if name.is_empty() {
            return AppUpdateOutcome {
                error: Some("凭据名称不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        if secret_ref.is_empty() {
            return AppUpdateOutcome {
                error: Some("安全引用不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        if let Some(group_id) = group_id {
            let group = self
                .storage
                .credential_groups
                .iter()
                .find(|group| group.id == group_id);
            if group.is_none() {
                return AppUpdateOutcome {
                    error: Some("密钥分组不存在".to_owned()),
                    ..AppUpdateOutcome::default()
                };
            }
            if group.is_some_and(|group| group.kind != kind) {
                return AppUpdateOutcome {
                    error: Some("密钥分组类型不匹配".to_owned()),
                    ..AppUpdateOutcome::default()
                };
            }
        }

        let key_algorithm = match kind {
            CredentialKind::PrivateKey | CredentialKind::Certificate => algorithm,
            CredentialKind::Password | CredentialKind::Agent => None,
        };

        self.storage.upsert_credential(CredentialMetadata {
            id: new_credential_id(),
            name: name.to_owned(),
            kind,
            group_id,
            username: None,
            secret: Some(SecretRef(secret_ref.to_owned())),
            key_algorithm,
            fingerprint: None,
        });

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    /// 更新凭据元数据，不修改已保存的敏感 payload。
    pub(in crate::model::app_state) fn update_credential_metadata(
        &mut self,
        original_name: &str,
        name: String,
        group_id: Option<CredentialGroupId>,
        algorithm: Option<KeyAlgorithm>,
    ) -> AppUpdateOutcome {
        let original_name = original_name.trim();
        let name = name.trim();

        if original_name.is_empty() || name.is_empty() {
            return AppUpdateOutcome {
                error: Some("凭据名称不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let Some(index) = self
            .storage
            .credentials
            .iter()
            .position(|credential| credential.name == original_name)
        else {
            return AppUpdateOutcome {
                error: Some(format!("找不到凭据：{original_name}")),
                ..AppUpdateOutcome::default()
            };
        };

        if name != original_name
            && self
                .storage
                .credentials
                .iter()
                .any(|credential| credential.name == name)
        {
            return AppUpdateOutcome {
                error: Some("凭据名称已存在".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let mut credential = self.storage.credentials[index].clone();
        if let Some(error) = validate_credential_group(
            &self.storage.credential_groups,
            group_id,
            credential.kind.clone(),
        ) {
            return AppUpdateOutcome {
                error: Some(error),
                ..AppUpdateOutcome::default()
            };
        }

        credential.name = name.to_owned();
        credential.group_id = group_id;
        credential.key_algorithm = match credential.kind {
            CredentialKind::PrivateKey | CredentialKind::Certificate => algorithm,
            CredentialKind::Password | CredentialKind::Agent => None,
        };
        self.storage.credentials[index] = credential;

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    /// 替换已保存凭据的本地 payload，不修改凭据名称、分组、算法等元数据。
    pub(in crate::model::app_state) fn update_credential_secret(
        &mut self,
        name: &str,
        secret_text: String,
    ) -> AppUpdateOutcome {
        let name = name.trim();
        if name.is_empty() {
            return AppUpdateOutcome {
                error: Some("凭据名称不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let Some(credential) = self
            .storage
            .credentials
            .iter()
            .find(|credential| credential.name == name)
        else {
            return AppUpdateOutcome {
                error: Some(format!("找不到凭据：{name}")),
                ..AppUpdateOutcome::default()
            };
        };

        let credential_id = credential.id;
        let credential_kind = credential.kind.clone();
        let Some(secret_ref) = credential.secret.clone() else {
            return AppUpdateOutcome {
                error: Some("该凭据没有可替换的本地内容".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        let Some(secret_kind) = local_secret_kind_for_credential(&credential_kind) else {
            return AppUpdateOutcome {
                error: Some("该凭据没有可替换的本地内容".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        let existing_secret = self
            .storage
            .secrets
            .iter()
            .find(|secret| secret.secret_ref == secret_ref);

        if existing_secret.is_some_and(|secret| {
            secret.kind != secret_kind
                || secret.encryption_version != 0
                || secret.encrypted_payload.is_none()
        }) {
            return AppUpdateOutcome {
                error: Some("当前内容暂不支持直接替换".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let payload = replacement_payload(&credential_kind, secret_text);
        if payload.is_empty() {
            return AppUpdateOutcome {
                error: Some("凭据内容不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }
        let inspection = inspect_credential_payload(credential_id, &credential_kind, &payload);
        if let Some(error) = credential_payload_validation_error(&credential_kind, &inspection) {
            return AppUpdateOutcome {
                error: Some(error),
                ..AppUpdateOutcome::default()
            };
        }

        self.storage.upsert_secret(SecretRecord::local_plaintext(
            secret_ref,
            secret_kind,
            payload,
        ));
        if let Some(credential) = self
            .storage
            .credentials
            .iter_mut()
            .find(|credential| credential.id == credential_id)
        {
            credential.key_algorithm = inspection.algorithm.clone();
            credential.fingerprint = inspection.fingerprint.clone();
        }
        self.storage.upsert_credential_inspection(inspection);

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    /// 将本地保存的凭据 payload 导出到文件。
    pub(in crate::model::app_state) fn export_credential_secret(
        &mut self,
        name: &str,
        target_path: &str,
    ) -> AppUpdateOutcome {
        let name = name.trim();
        let target_path = target_path.trim();

        if target_path.is_empty() {
            return AppUpdateOutcome {
                error: Some("导出路径不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let Some(credential) = self
            .storage
            .credentials
            .iter()
            .find(|credential| credential.name == name)
        else {
            return AppUpdateOutcome {
                error: Some(format!("找不到凭据：{name}")),
                ..AppUpdateOutcome::default()
            };
        };
        let Some(secret_ref) = credential.secret.as_ref() else {
            return AppUpdateOutcome {
                error: Some("凭据没有可导出的安全引用".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };
        let Some(secret) = self
            .storage
            .secrets
            .iter()
            .find(|secret| &secret.secret_ref == secret_ref)
        else {
            return AppUpdateOutcome {
                error: Some("找不到安全存储内容".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        if secret.encryption_version != 0 {
            return AppUpdateOutcome {
                error: Some("当前版本暂不支持导出已加密内容".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let Some(payload) = secret.encrypted_payload.as_ref() else {
            return AppUpdateOutcome {
                error: Some("安全存储内容为空，无法导出".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        let path = Path::new(target_path);
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            if let Err(error) = std::fs::create_dir_all(parent) {
                return AppUpdateOutcome {
                    error: Some(format!("创建导出目录失败：{error}")),
                    ..AppUpdateOutcome::default()
                };
            }
        }
        if let Err(error) = std::fs::write(path, payload) {
            return AppUpdateOutcome {
                error: Some(format!("写入导出文件失败：{error}")),
                ..AppUpdateOutcome::default()
            };
        }

        AppUpdateOutcome::default()
    }

    /// 复制一个已保存的凭据元数据。
    pub(in crate::model::app_state) fn duplicate_credential(
        &mut self,
        name: &str,
    ) -> AppUpdateOutcome {
        let Some(source) = self
            .storage
            .credentials
            .iter()
            .find(|credential| credential.name == name)
            .cloned()
        else {
            return AppUpdateOutcome {
                error: Some(format!("找不到凭据：{name}")),
                ..AppUpdateOutcome::default()
            };
        };

        let mut duplicate = source;
        duplicate.id = new_credential_id();
        let original_secret_ref = duplicate.secret.clone();
        duplicate.name = next_credential_copy_name(&self.storage.credentials, &duplicate.name);

        let mut duplicate_inspection = None;
        if let Some(secret_ref) = original_secret_ref.as_ref() {
            if let Some(source_secret) = self
                .storage
                .secrets
                .iter()
                .find(|secret| &secret.secret_ref == secret_ref)
                .cloned()
            {
                let (namespace, fallback) = credential_secret_namespace(&duplicate.kind);
                let copied_ref = next_secret_ref(self, namespace, fallback, &duplicate.name);
                let mut copied_secret = source_secret;
                copied_secret.secret_ref = copied_ref.clone();
                if let Some(payload) = copied_secret.encrypted_payload.as_ref() {
                    duplicate_inspection = Some(inspect_credential_payload(
                        duplicate.id,
                        &duplicate.kind,
                        payload,
                    ));
                }
                self.storage.upsert_secret(copied_secret);
                duplicate.secret = Some(copied_ref);
            }
        }

        self.storage.upsert_credential(duplicate);
        if let Some(inspection) = duplicate_inspection {
            self.storage.upsert_credential_inspection(inspection);
        }
        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    /// 删除一个已保存的凭据元数据。
    pub(in crate::model::app_state) fn remove_credential(
        &mut self,
        name: &str,
    ) -> AppUpdateOutcome {
        // 删除失败时返回用户可见错误，不静默忽略，方便设置页提示配置已经过期。
        let secret_ref = self
            .storage
            .credentials
            .iter()
            .find(|credential| credential.name == name)
            .and_then(|credential| credential.secret.clone());

        if self.storage.remove_credential(name) {
            if let Some(secret_ref) = secret_ref.as_ref() {
                if !self.secret_ref_is_still_referenced(secret_ref) {
                    self.storage.remove_secret(secret_ref);
                }
            }
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            AppUpdateOutcome {
                error: Some(format!("找不到凭据：{name}")),
                ..AppUpdateOutcome::default()
            }
        }
    }

    /// 移动凭据到同类型分组或该类型根节点。
    pub(in crate::model::app_state) fn move_credential(
        &mut self,
        name: &str,
        group_id: Option<CredentialGroupId>,
    ) -> AppUpdateOutcome {
        let Some(index) = self
            .storage
            .credentials
            .iter()
            .position(|credential| credential.name == name)
        else {
            return AppUpdateOutcome {
                error: Some(format!("找不到凭据：{name}")),
                ..AppUpdateOutcome::default()
            };
        };

        let kind = self.storage.credentials[index].kind.clone();
        if let Some(error) =
            validate_credential_group(&self.storage.credential_groups, group_id, kind)
        {
            return AppUpdateOutcome {
                error: Some(error),
                ..AppUpdateOutcome::default()
            };
        }

        if self.storage.credentials[index].group_id == group_id {
            return AppUpdateOutcome::default();
        }

        self.storage.credentials[index].group_id = group_id;
        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    fn secret_ref_is_still_referenced(&self, secret_ref: &SecretRef) -> bool {
        self.storage.credentials.iter().any(|credential| {
            credential
                .secret
                .as_ref()
                .is_some_and(|reference| reference == secret_ref)
        }) || self
            .storage
            .hosts
            .iter()
            .any(|host| auth_profile_uses_secret_ref(&host.auth, secret_ref))
    }

    pub(super) fn local_private_key_from_secret_ref(
        &self,
        secret_ref: &str,
        display_name: &str,
    ) -> Result<PrivateKey, String> {
        if secret_ref.is_empty() {
            return Err(format!("{display_name}不能为空"));
        }

        let Some(credential) = self.storage.credentials.iter().find(|credential| {
            credential.kind == CredentialKind::PrivateKey
                && credential
                    .secret
                    .as_ref()
                    .is_some_and(|reference| reference.0 == secret_ref)
        }) else {
            return Err(format!("{display_name}不存在或不是私钥凭据"));
        };
        let Some(secret_ref) = credential.secret.as_ref() else {
            return Err(format!("{display_name}没有本地内容引用"));
        };
        let Some(secret) = self
            .storage
            .secrets
            .iter()
            .find(|secret| &secret.secret_ref == secret_ref)
        else {
            return Err(format!("{display_name}没有保存的本地内容"));
        };

        if secret.kind != SecretMaterialKind::PrivateKey {
            return Err(format!("{display_name}内容类型不是私钥"));
        }
        if secret.encryption_version != 0 {
            return Err(format!("{display_name}当前已加密，暂不支持直接签发证书"));
        }

        let Some(payload) = secret.encrypted_payload.as_ref() else {
            return Err(format!("{display_name}内容为空"));
        };
        decode_plaintext_private_key(payload, display_name)
    }
}
