//! 证书类凭据材料生成。

use crate::model::{
    CredentialGroupId, CredentialKind, CredentialMetadata, KeyAlgorithm, QuickHostAuthField,
    SecretMaterialKind, SecretRecord,
};
use russh::keys::{
    HashAlg,
    ssh_key::{certificate, rand_core::OsRng},
};

use super::super::{AppState, AppUpdateOutcome};
use super::credential_certificate_params::{
    current_unix_seconds, parse_certificate_principals, parse_certificate_serial,
    parse_certificate_type, parse_certificate_valid_days,
};
use super::credential_groups::validate_credential_group;
use super::credential_ids::new_credential_id;
use super::credential_payload::inspect_credential_payload;
use super::credential_refs::next_secret_ref;

impl AppState {
    /// 用已保存的 CA 私钥签发 OpenSSH 用户/主机证书，并保存为证书凭据。
    pub(in crate::model::app_state) fn generate_certificate_credential(
        &mut self,
        name: String,
        group_id: Option<CredentialGroupId>,
        ca_private_key_ref: String,
        subject_private_key_ref: String,
        cert_type: String,
        principals: String,
        valid_days: String,
        key_id: String,
        serial: String,
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
            CredentialKind::Certificate,
        ) {
            return AppUpdateOutcome {
                error: Some(error),
                ..AppUpdateOutcome::default()
            };
        }

        let cert_type = match parse_certificate_type(&cert_type) {
            Ok(cert_type) => cert_type,
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(error),
                    ..AppUpdateOutcome::default()
                };
            }
        };
        let principals = match parse_certificate_principals(&principals) {
            Ok(principals) => principals,
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(error),
                    ..AppUpdateOutcome::default()
                };
            }
        };
        let valid_days = match parse_certificate_valid_days(&valid_days) {
            Ok(valid_days) => valid_days,
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(error),
                    ..AppUpdateOutcome::default()
                };
            }
        };
        let serial = match parse_certificate_serial(&serial) {
            Ok(serial) => serial,
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(error),
                    ..AppUpdateOutcome::default()
                };
            }
        };

        let ca_private_key =
            match self.local_private_key_from_secret_ref(ca_private_key_ref.trim(), "CA 私钥") {
                Ok(private_key) => private_key,
                Err(error) => {
                    return AppUpdateOutcome {
                        error: Some(error),
                        ..AppUpdateOutcome::default()
                    };
                }
            };
        let subject_private_key = match self
            .local_private_key_from_secret_ref(subject_private_key_ref.trim(), "主体私钥")
        {
            Ok(private_key) => private_key,
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(error),
                    ..AppUpdateOutcome::default()
                };
            }
        };

        let valid_after = match current_unix_seconds() {
            Ok(seconds) => seconds,
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(error),
                    ..AppUpdateOutcome::default()
                };
            }
        };
        let valid_seconds = valid_days.saturating_mul(86_400);
        let Some(valid_before) = valid_after.checked_add(valid_seconds) else {
            return AppUpdateOutcome {
                error: Some("证书有效期超出可用时间范围".to_owned()),
                ..AppUpdateOutcome::default()
            };
        };

        let mut rng = OsRng;
        let mut builder = match certificate::Builder::new_with_random_nonce(
            &mut rng,
            subject_private_key.public_key(),
            valid_after,
            valid_before,
        ) {
            Ok(builder) => builder,
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(format!("创建证书失败：{error}")),
                    ..AppUpdateOutcome::default()
                };
            }
        };

        if let Err(error) = builder.serial(serial) {
            return AppUpdateOutcome {
                error: Some(format!("设置证书序列号失败：{error}")),
                ..AppUpdateOutcome::default()
            };
        }
        if let Err(error) = builder.cert_type(cert_type) {
            return AppUpdateOutcome {
                error: Some(format!("设置证书类型失败：{error}")),
                ..AppUpdateOutcome::default()
            };
        }
        let key_id = key_id.trim();
        if let Err(error) = builder.key_id(if key_id.is_empty() { name } else { key_id }) {
            return AppUpdateOutcome {
                error: Some(format!("设置证书 Key ID 失败：{error}")),
                ..AppUpdateOutcome::default()
            };
        }
        if let Err(error) = builder.comment(name) {
            return AppUpdateOutcome {
                error: Some(format!("设置证书备注失败：{error}")),
                ..AppUpdateOutcome::default()
            };
        }
        for principal in principals {
            if let Err(error) = builder.valid_principal(principal) {
                return AppUpdateOutcome {
                    error: Some(format!("设置 Principal 失败：{error}")),
                    ..AppUpdateOutcome::default()
                };
            }
        }

        let certificate = match builder.sign(&ca_private_key) {
            Ok(certificate) => certificate,
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(format!("签发证书失败：{error}")),
                    ..AppUpdateOutcome::default()
                };
            }
        };
        let mut payload = match certificate.to_openssh() {
            Ok(payload) => payload.into_bytes(),
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(format!("编码证书失败：{error}")),
                    ..AppUpdateOutcome::default()
                };
            }
        };
        if !payload.ends_with(b"\n") {
            payload.push(b'\n');
        }

        let fingerprint = subject_private_key
            .public_key()
            .fingerprint(HashAlg::Sha256)
            .to_string();
        let algorithm = KeyAlgorithm::from_ssh_algorithm(subject_private_key.algorithm().as_str());
        let secret_ref = next_secret_ref(self, "certs", "certificate", name);
        let credential_id = new_credential_id();
        let inspection =
            inspect_credential_payload(credential_id, &CredentialKind::Certificate, &payload);

        self.storage.upsert_secret(SecretRecord::local_plaintext(
            secret_ref.clone(),
            SecretMaterialKind::Certificate,
            payload,
        ));
        self.storage.upsert_credential(CredentialMetadata {
            id: credential_id,
            name: name.to_owned(),
            kind: CredentialKind::Certificate,
            group_id,
            username: None,
            secret: Some(secret_ref.clone()),
            key_algorithm: Some(algorithm),
            fingerprint: Some(fingerprint),
        });
        self.storage.upsert_credential_inspection(inspection);
        self.ui
            .set_quick_host_auth_field(QuickHostAuthField::CertificateRef, &secret_ref.0);

        AppUpdateOutcome {
            state_changed: true,
            ..AppUpdateOutcome::default()
        }
    }
}
