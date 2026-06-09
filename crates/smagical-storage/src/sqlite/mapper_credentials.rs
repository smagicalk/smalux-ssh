use sea_orm::{ActiveValue::Set, DatabaseConnection, EntityTrait, QueryOrder};
use uuid::Uuid;

use smagical_core::{
    CertificateInspection, CredentialGroup, CredentialGroupId, CredentialId, CredentialInspection,
    CredentialMetadata, SecretRecord, SecretRef,
};

use super::mapper_common::*;
use super::{current_unix_secs, entity};
use crate::StoragePersistenceError;

pub(super) async fn load_credentials(
    db: &DatabaseConnection,
) -> Result<Vec<CredentialMetadata>, StoragePersistenceError> {
    // 凭据表只保存可展示元数据和 SecretRef，真实 secret payload 由 secrets 表单独加载。
    entity::credential::Entity::find()
        .order_by_asc(entity::credential::Column::Name)
        .all(db)
        .await?
        .into_iter()
        .map(|model| {
            let id = model
                .id
                .as_deref()
                .map(parse_uuid)
                .transpose()?
                .map(CredentialId)
                .unwrap_or_else(|| CredentialId(Uuid::new_v4()));
            Ok(CredentialMetadata {
                id,
                name: model.name,
                kind: credential_kind_from_str(&model.kind),
                group_id: model
                    .group_id
                    .as_deref()
                    .map(parse_uuid)
                    .transpose()?
                    .map(CredentialGroupId),
                username: model.username,
                secret: model.secret_ref.map(SecretRef),
                key_algorithm: model
                    .key_algorithm
                    .as_deref()
                    .map(|kind| key_algorithm_from_parts(kind, model.key_algorithm_raw.as_deref())),
                fingerprint: model.fingerprint,
            })
        })
        .collect()
}

pub(super) async fn load_credential_groups(
    db: &DatabaseConnection,
) -> Result<Vec<CredentialGroup>, StoragePersistenceError> {
    entity::credential_group::Entity::find()
        .order_by_asc(entity::credential_group::Column::SortOrder)
        .order_by_asc(entity::credential_group::Column::Name)
        .all(db)
        .await?
        .into_iter()
        .map(|model| {
            Ok(CredentialGroup {
                id: CredentialGroupId(parse_uuid(&model.id)?),
                name: model.name,
                kind: credential_kind_from_str(&model.kind),
                parent_id: model
                    .parent_id
                    .as_deref()
                    .map(parse_uuid)
                    .transpose()?
                    .map(CredentialGroupId),
                sort_order: model.sort_order,
            })
        })
        .collect()
}

pub(super) async fn load_credential_inspections(
    db: &DatabaseConnection,
) -> Result<Vec<CredentialInspection>, StoragePersistenceError> {
    entity::credential_inspection::Entity::find()
        .order_by_asc(entity::credential_inspection::Column::CredentialId)
        .all(db)
        .await?
        .into_iter()
        .map(|model| {
            let certificate = if model.cert_type.is_some()
                || model.serial.is_some()
                || model.key_id.is_some()
                || model.principals_text.is_some()
            {
                Some(CertificateInspection {
                    cert_type: model.cert_type,
                    serial: model.serial.and_then(|serial| serial.parse::<u64>().ok()),
                    key_id: model.key_id,
                    principals: principals_from_text(model.principals_text.as_deref()),
                    valid_after_unix_secs: model
                        .valid_after_unix_secs
                        .and_then(|value| u64::try_from(value).ok()),
                    valid_before_unix_secs: model
                        .valid_before_unix_secs
                        .and_then(|value| u64::try_from(value).ok()),
                    ca_fingerprint: model.ca_fingerprint,
                    subject_fingerprint: model.subject_fingerprint,
                    critical_options_json: model.critical_options_json,
                    extensions_json: model.extensions_json,
                })
            } else {
                None
            };

            Ok(CredentialInspection {
                credential_id: CredentialId(parse_uuid(&model.credential_id)?),
                kind: credential_kind_from_str(&model.kind),
                payload_hash: model.payload_hash,
                parser_version: model.parser_version,
                parse_error: model.parse_error,
                algorithm: model
                    .key_algorithm
                    .as_deref()
                    .map(|kind| key_algorithm_from_parts(kind, model.key_algorithm_raw.as_deref())),
                fingerprint: model.fingerprint,
                public_key: model.public_key,
                comment: model.comment,
                encrypted: model.encrypted,
                password_length: model
                    .password_length
                    .and_then(|value| usize::try_from(value).ok()),
                certificate,
            })
        })
        .collect()
}

pub(super) async fn save_credential_groups(
    db: &DatabaseConnection,
    groups: &[CredentialGroup],
) -> Result<(), StoragePersistenceError> {
    let now = current_unix_secs();
    for group in groups {
        entity::credential_group::Entity::insert(entity::credential_group::ActiveModel {
            id: Set(group.id.0.to_string()),
            name: Set(group.name.clone()),
            kind: Set(credential_kind_to_str(&group.kind).to_owned()),
            parent_id: Set(group.parent_id.map(|parent_id| parent_id.0.to_string())),
            sort_order: Set(group.sort_order),
            created_at_unix_secs: Set(now),
            updated_at_unix_secs: Set(now),
        })
        .exec(db)
        .await?;
    }
    Ok(())
}

pub(super) async fn save_credential_inspections(
    db: &DatabaseConnection,
    inspections: &[CredentialInspection],
) -> Result<(), StoragePersistenceError> {
    let now = current_unix_secs();
    for inspection in inspections {
        let (key_algorithm, key_algorithm_raw) = inspection
            .algorithm
            .as_ref()
            .map(key_algorithm_to_parts)
            .unwrap_or((None, None));
        let certificate = inspection.certificate.as_ref();
        entity::credential_inspection::Entity::insert(entity::credential_inspection::ActiveModel {
            credential_id: Set(inspection.credential_id.0.to_string()),
            kind: Set(credential_kind_to_str(&inspection.kind).to_owned()),
            payload_hash: Set(inspection.payload_hash.clone()),
            parser_version: Set(inspection.parser_version),
            parse_error: Set(inspection.parse_error.clone()),
            key_algorithm: Set(key_algorithm),
            key_algorithm_raw: Set(key_algorithm_raw),
            fingerprint: Set(inspection.fingerprint.clone()),
            public_key: Set(inspection.public_key.clone()),
            comment: Set(inspection.comment.clone()),
            encrypted: Set(inspection.encrypted),
            password_length: Set(inspection
                .password_length
                .and_then(|value| i32::try_from(value).ok())),
            cert_type: Set(certificate.and_then(|certificate| certificate.cert_type.clone())),
            serial: Set(certificate
                .and_then(|certificate| certificate.serial)
                .map(|serial| serial.to_string())),
            key_id: Set(certificate.and_then(|certificate| certificate.key_id.clone())),
            principals_text: Set(
                certificate.and_then(|certificate| principals_to_text(&certificate.principals))
            ),
            valid_after_unix_secs: Set(certificate
                .and_then(|certificate| certificate.valid_after_unix_secs)
                .and_then(|value| i64::try_from(value).ok())),
            valid_before_unix_secs: Set(certificate
                .and_then(|certificate| certificate.valid_before_unix_secs)
                .and_then(|value| i64::try_from(value).ok())),
            ca_fingerprint: Set(
                certificate.and_then(|certificate| certificate.ca_fingerprint.clone())
            ),
            subject_fingerprint: Set(
                certificate.and_then(|certificate| certificate.subject_fingerprint.clone())
            ),
            critical_options_json: Set(
                certificate.and_then(|certificate| certificate.critical_options_json.clone())
            ),
            extensions_json: Set(
                certificate.and_then(|certificate| certificate.extensions_json.clone())
            ),
            created_at_unix_secs: Set(now),
            updated_at_unix_secs: Set(now),
        })
        .exec(db)
        .await?;
    }
    Ok(())
}

pub(super) async fn save_credentials(
    db: &DatabaseConnection,
    credentials: &[CredentialMetadata],
) -> Result<(), StoragePersistenceError> {
    let now = current_unix_secs();
    for credential in credentials {
        // key_algorithm 分成标准 kind 和 raw，既能查询常见算法，也保留未知算法原文。
        let (key_algorithm, key_algorithm_raw) = credential
            .key_algorithm
            .as_ref()
            .map(key_algorithm_to_parts)
            .unwrap_or((None, None));
        entity::credential::Entity::insert(entity::credential::ActiveModel {
            name: Set(credential.name.clone()),
            id: Set(Some(credential.id.0.to_string())),
            kind: Set(credential_kind_to_str(&credential.kind).to_owned()),
            group_id: Set(credential.group_id.map(|group_id| group_id.0.to_string())),
            username: Set(credential.username.clone()),
            secret_ref: Set(credential
                .secret
                .as_ref()
                .map(|reference| reference.0.clone())),
            key_algorithm: Set(key_algorithm),
            key_algorithm_raw: Set(key_algorithm_raw),
            fingerprint: Set(credential.fingerprint.clone()),
            created_at_unix_secs: Set(now),
            updated_at_unix_secs: Set(now),
        })
        .exec(db)
        .await?;
    }
    Ok(())
}

pub(super) async fn load_secrets(
    db: &DatabaseConnection,
) -> Result<Vec<SecretRecord>, StoragePersistenceError> {
    entity::secret::Entity::find()
        .order_by_asc(entity::secret::Column::SecretRef)
        .all(db)
        .await?
        .into_iter()
        .map(|model| {
            Ok(SecretRecord {
                secret_ref: SecretRef(model.secret_ref),
                kind: secret_material_kind_from_str(&model.secret_kind),
                encryption_version: model.encryption_version,
                kdf: model.kdf,
                kdf_params_toml: model.kdf_params_toml,
                salt: model.salt,
                nonce: model.nonce,
                encrypted_payload: model.encrypted_payload,
                external_store: model.external_store,
                external_key: model.external_key,
            })
        })
        .collect()
}

pub(super) async fn save_secrets(
    db: &DatabaseConnection,
    secrets: &[SecretRecord],
) -> Result<(), StoragePersistenceError> {
    let now = current_unix_secs();
    for secret in secrets {
        entity::secret::Entity::insert(entity::secret::ActiveModel {
            secret_ref: Set(secret.secret_ref.0.clone()),
            secret_kind: Set(secret_material_kind_to_string(&secret.kind)),
            encryption_version: Set(secret.encryption_version),
            kdf: Set(secret.kdf.clone()),
            kdf_params_toml: Set(secret.kdf_params_toml.clone()),
            salt: Set(secret.salt.clone()),
            nonce: Set(secret.nonce.clone()),
            encrypted_payload: Set(secret.encrypted_payload.clone()),
            external_store: Set(secret.external_store.clone()),
            external_key: Set(secret.external_key.clone()),
            created_at_unix_secs: Set(now),
            updated_at_unix_secs: Set(now),
        })
        .exec(db)
        .await?;
    }
    Ok(())
}
