//! 凭据材料的本地生成与保存。

use crate::core::CoreState;
use crate::model::{
    CredentialGroupId, CredentialKind, CredentialMetadata, KeyAlgorithm, SecretMaterialKind,
    SecretRecord,
};
use russh::keys::{HashAlg, ssh_key::LineEnding};

use super::super::AppUpdateOutcome;
use super::credential_groups::validate_credential_group;
use super::credential_ids::new_credential_id;
use super::credential_payload::{generate_private_key, inspect_credential_payload};
use super::credential_refs::next_secret_ref;

impl CoreState {
    /// 生成新的 OpenSSH 私钥，并创建对应的凭据元数据。
    pub(crate) fn generate_private_key_credential(
        &mut self,
        name: String,
        group_id: Option<CredentialGroupId>,
        algorithm: Option<KeyAlgorithm>,
    ) -> AppUpdateOutcome {
        let name = name.trim();

        if name.is_empty() {
            return AppUpdateOutcome {
                error: Some("凭据名称不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        if let Some(error) = validate_credential_group(
            &self.storage.credential_groups,
            group_id,
            CredentialKind::PrivateKey,
        ) {
            return AppUpdateOutcome {
                error: Some(error),
                ..AppUpdateOutcome::default()
            };
        }

        let key_algorithm = algorithm.unwrap_or(KeyAlgorithm::Ed25519);
        if matches!(key_algorithm, KeyAlgorithm::Unknown(_)) {
            return AppUpdateOutcome {
                error: Some("当前算法暂不支持生成私钥".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        let private_key = match generate_private_key(name, &key_algorithm) {
            Ok(private_key) => private_key,
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(format!("生成私钥失败：{error}")),
                    ..AppUpdateOutcome::default()
                };
            }
        };

        let payload = match private_key.to_openssh(LineEnding::LF) {
            Ok(payload) => payload.as_bytes().to_vec(),
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(format!("编码私钥失败：{error}")),
                    ..AppUpdateOutcome::default()
                };
            }
        };
        let fingerprint = private_key
            .public_key()
            .fingerprint(HashAlg::Sha256)
            .to_string();
        let secret_ref = next_secret_ref(self, "keys", "private-key", name);
        let credential_id = new_credential_id();
        let inspection =
            inspect_credential_payload(credential_id, &CredentialKind::PrivateKey, &payload);

        self.storage.upsert_secret(SecretRecord::local_plaintext(
            secret_ref.clone(),
            SecretMaterialKind::PrivateKey,
            payload,
        ));
        self.storage.upsert_credential(CredentialMetadata {
            id: credential_id,
            name: name.to_owned(),
            kind: CredentialKind::PrivateKey,
            group_id,
            username: None,
            secret: Some(secret_ref.clone()),
            key_algorithm: Some(key_algorithm),
            fingerprint: Some(fingerprint),
        });
        self.storage.upsert_credential_inspection(inspection);
        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }

    /// 保存密码到本地 Secret 存储，并创建对应的凭据元数据。
    pub(crate) fn save_password_credential(
        &mut self,
        name: String,
        group_id: Option<CredentialGroupId>,
        password: String,
    ) -> AppUpdateOutcome {
        let name = name.trim();

        if name.is_empty() {
            return AppUpdateOutcome {
                error: Some("凭据名称不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        if password.is_empty() {
            return AppUpdateOutcome {
                error: Some("密码不能为空".to_owned()),
                ..AppUpdateOutcome::default()
            };
        }

        if let Some(error) = validate_credential_group(
            &self.storage.credential_groups,
            group_id,
            CredentialKind::Password,
        ) {
            return AppUpdateOutcome {
                error: Some(error),
                ..AppUpdateOutcome::default()
            };
        }

        let secret_ref = next_secret_ref(self, "passwords", "password", name);
        let payload = password.into_bytes();
        let credential_id = new_credential_id();
        let inspection =
            inspect_credential_payload(credential_id, &CredentialKind::Password, &payload);
        self.storage.upsert_secret(SecretRecord::local_plaintext(
            secret_ref.clone(),
            SecretMaterialKind::Password,
            payload,
        ));
        self.storage.upsert_credential(CredentialMetadata {
            id: credential_id,
            name: name.to_owned(),
            kind: CredentialKind::Password,
            group_id,
            username: None,
            secret: Some(secret_ref.clone()),
            key_algorithm: None,
            fingerprint: None,
        });
        self.storage.upsert_credential_inspection(inspection);
        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }
}
