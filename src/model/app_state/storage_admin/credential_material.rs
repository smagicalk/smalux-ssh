use crate::core::CoreState;
use std::path::Path;

use crate::model::{
    CredentialGroupId, CredentialKind, CredentialMetadata, KeyAlgorithm, QuickHostAuthField,
    SecretMaterialKind, SecretRecord,
};

use super::super::AppUpdateOutcome;
use super::credential_groups::validate_credential_group;
use super::credential_ids::new_credential_id;
use super::credential_payload::{credential_payload_validation_error, inspect_credential_payload};
use super::credential_refs::next_secret_ref;

impl CoreState {
    /// 从本地文件导入私钥内容，并创建对应的凭据元数据。
    pub(crate) fn import_private_key_credential(
        &mut self,
        name: String,
        group_id: Option<CredentialGroupId>,
        source_path: String,
        algorithm: Option<KeyAlgorithm>,
    ) -> AppUpdateOutcome {
        self.import_file_backed_credential(
            CredentialKind::PrivateKey,
            SecretMaterialKind::PrivateKey,
            QuickHostAuthField::PrivateKeyRef,
            "keys",
            "private-key",
            "私钥",
            name,
            group_id,
            source_path,
            algorithm,
        )
    }

    /// 从用户粘贴的 OpenSSH 私钥文本导入内容，并创建对应的凭据元数据。
    pub(crate) fn import_private_key_text_credential(
        &mut self,
        name: String,
        group_id: Option<CredentialGroupId>,
        private_key_text: String,
        algorithm: Option<KeyAlgorithm>,
    ) -> AppUpdateOutcome {
        self.import_text_backed_credential(
            CredentialKind::PrivateKey,
            SecretMaterialKind::PrivateKey,
            QuickHostAuthField::PrivateKeyRef,
            "keys",
            "private-key",
            name,
            group_id,
            private_key_text,
            algorithm,
            "私钥文本不能为空",
        )
    }

    /// 从用户粘贴的 OpenSSH 证书文本导入内容，并创建对应的凭据元数据。
    pub(crate) fn import_certificate_text_credential(
        &mut self,
        name: String,
        group_id: Option<CredentialGroupId>,
        certificate_text: String,
        algorithm: Option<KeyAlgorithm>,
    ) -> AppUpdateOutcome {
        self.import_text_backed_credential(
            CredentialKind::Certificate,
            SecretMaterialKind::Certificate,
            QuickHostAuthField::CertificateRef,
            "certs",
            "certificate",
            name,
            group_id,
            certificate_text,
            algorithm,
            "证书文本不能为空",
        )
    }

    /// 从本地文件导入 OpenSSH 证书内容，并创建对应的凭据元数据。
    pub(crate) fn import_certificate_credential(
        &mut self,
        name: String,
        group_id: Option<CredentialGroupId>,
        source_path: String,
        algorithm: Option<KeyAlgorithm>,
    ) -> AppUpdateOutcome {
        self.import_file_backed_credential(
            CredentialKind::Certificate,
            SecretMaterialKind::Certificate,
            QuickHostAuthField::CertificateRef,
            "certs",
            "certificate",
            "证书",
            name,
            group_id,
            source_path,
            algorithm,
        )
    }

    fn import_text_backed_credential(
        &mut self,
        kind: CredentialKind,
        secret_kind: SecretMaterialKind,
        auth_field: QuickHostAuthField,
        secret_namespace: &str,
        secret_fallback: &str,
        name: String,
        group_id: Option<CredentialGroupId>,
        secret_text: String,
        algorithm: Option<KeyAlgorithm>,
        empty_secret_error: &str,
    ) -> AppUpdateOutcome {
        let name = name.trim();
        let secret_text = secret_text.trim();

        if name.is_empty() {
            return AppUpdateOutcome {
                error: Some("凭据名称不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        if secret_text.is_empty() {
            return AppUpdateOutcome {
                error: Some(empty_secret_error.to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        if let Some(error) =
            validate_credential_group(&self.storage.credential_groups, group_id, kind.clone())
        {
            return AppUpdateOutcome {
                error: Some(error),
                ..AppUpdateOutcome::default()
            };
        }

        // PEM/OpenSSH 文本通常以换行结束；这里保留内容语义，同时避免导出内容不一致。
        let mut payload = secret_text.as_bytes().to_vec();
        if !payload.ends_with(b"\n") {
            payload.push(b'\n');
        }

        self.save_imported_credential(
            kind,
            secret_kind,
            auth_field,
            secret_namespace,
            secret_fallback,
            name,
            group_id,
            payload,
            algorithm,
        )
    }

    fn import_file_backed_credential(
        &mut self,
        kind: CredentialKind,
        secret_kind: SecretMaterialKind,
        auth_field: QuickHostAuthField,
        secret_namespace: &str,
        secret_fallback: &str,
        display_name: &str,
        name: String,
        group_id: Option<CredentialGroupId>,
        source_path: String,
        algorithm: Option<KeyAlgorithm>,
    ) -> AppUpdateOutcome {
        let name = name.trim();
        let source_path = source_path.trim();

        if name.is_empty() {
            return AppUpdateOutcome {
                error: Some("凭据名称不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        if source_path.is_empty() {
            return AppUpdateOutcome {
                error: Some(format!("{display_name}文件路径不能为空")),
                ..AppUpdateOutcome::default()
            };
        }

        if let Some(error) =
            validate_credential_group(&self.storage.credential_groups, group_id, kind.clone())
        {
            return AppUpdateOutcome {
                error: Some(error),
                ..AppUpdateOutcome::default()
            };
        }

        let payload = match std::fs::read(Path::new(source_path)) {
            Ok(payload) => payload,
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(format!("读取{display_name}失败：{error}")),
                    ..AppUpdateOutcome::default()
                };
            }
        };

        if payload.is_empty() {
            return AppUpdateOutcome {
                error: Some(format!("{display_name}文件不能为空")),
                ..AppUpdateOutcome::default()
            };
        }

        self.save_imported_credential(
            kind,
            secret_kind,
            auth_field,
            secret_namespace,
            secret_fallback,
            name,
            group_id,
            payload,
            algorithm,
        )
    }

    fn save_imported_credential(
        &mut self,
        kind: CredentialKind,
        secret_kind: SecretMaterialKind,
        _auth_field: QuickHostAuthField,
        secret_namespace: &str,
        secret_fallback: &str,
        name: &str,
        group_id: Option<CredentialGroupId>,
        payload: Vec<u8>,
        algorithm: Option<KeyAlgorithm>,
    ) -> AppUpdateOutcome {
        let secret_ref = next_secret_ref(self, secret_namespace, secret_fallback, name);
        let credential_id = new_credential_id();
        let inspection = inspect_credential_payload(credential_id, &kind, &payload);
        if let Some(error) = credential_payload_validation_error(&kind, &inspection) {
            return AppUpdateOutcome {
                error: Some(error),
                ..AppUpdateOutcome::default()
            };
        }
        let key_algorithm = inspection.algorithm.clone().or(algorithm);
        self.storage.upsert_secret(SecretRecord::local_plaintext(
            secret_ref.clone(),
            secret_kind,
            payload,
        ));
        self.storage.upsert_credential(CredentialMetadata {
            id: credential_id,
            name: name.to_owned(),
            kind,
            group_id,
            username: None,
            secret: Some(secret_ref.clone()),
            key_algorithm,
            fingerprint: None,
        });
        self.storage.upsert_credential_inspection(inspection);
        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }
}
