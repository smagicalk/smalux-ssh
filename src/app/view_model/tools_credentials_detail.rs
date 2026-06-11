//! 凭据详情字段展示模型。

use crate::model::{
    AppState, CredentialGroup, CredentialInspection, CredentialKind, CredentialMetadata,
    SecretRecord,
};

use super::i18n::{Locale, locale_for_state, tr};
use super::tools_credentials_common::{
    credential_group_path, credential_kind_label, credential_row_id, credential_secret_available,
    credential_storage_label, credential_visible_in_security, key_algorithm_label,
};
use super::tools_types::CredentialDetailFieldViewModel;

pub(in crate::app::view_model) fn credential_detail_fields(
    state: &AppState,
) -> Vec<CredentialDetailFieldViewModel> {
    let locale = locale_for_state(state);
    let empty = tr(locale, "tool.empty_value");
    let mut rows = Vec::new();

    for credential in state
        .storage
        .credentials
        .iter()
        .filter(|credential| credential_visible_in_security(&credential.kind))
    {
        append_credential_detail_fields(
            &mut rows,
            credential,
            state
                .storage
                .credential_inspections
                .iter()
                .find(|inspection| inspection.credential_id == credential.id),
            &state.storage.credential_groups,
            &state.storage.secrets,
            locale,
            empty,
        );
    }

    rows
}

fn append_credential_detail_fields(
    rows: &mut Vec<CredentialDetailFieldViewModel>,
    credential: &CredentialMetadata,
    inspection: Option<&CredentialInspection>,
    groups: &[CredentialGroup],
    secrets: &[SecretRecord],
    locale: Locale,
    empty: &str,
) {
    let credential_id = credential_row_id(credential);
    let secret_available = credential_secret_available(credential, secrets);
    push_credential_detail_field(
        rows,
        &credential_id,
        tr(locale, "security.field_type"),
        credential_kind_label(&credential.kind, locale).to_owned(),
    );
    if let Some(username) = credential
        .username
        .as_ref()
        .filter(|username| !username.is_empty())
    {
        push_credential_detail_field(
            rows,
            &credential_id,
            tr(locale, "security.field_username"),
            username.clone(),
        );
    }
    push_credential_detail_field(
        rows,
        &credential_id,
        tr(locale, "security.field_group"),
        credential_group_path(groups, credential.group_id, &credential.kind, locale),
    );
    push_credential_detail_field(
        rows,
        &credential_id,
        tr(locale, "security.field_secret_ref"),
        credential_storage_label(credential, secret_available, locale, empty),
    );

    let algorithm = inspection
        .and_then(|inspection| inspection.algorithm.as_ref())
        .or(credential.key_algorithm.as_ref())
        .map(key_algorithm_label);
    push_optional_credential_detail_field(
        rows,
        &credential_id,
        tr(locale, "security.field_algorithm"),
        algorithm,
    );
    let fingerprint = inspection
        .and_then(|inspection| inspection.fingerprint.clone())
        .or_else(|| credential.fingerprint.clone());
    push_optional_credential_detail_field(
        rows,
        &credential_id,
        tr(locale, "security.field_fingerprint"),
        fingerprint,
    );

    if let Some(parse_error) = inspection.and_then(|inspection| inspection.parse_error.clone()) {
        push_credential_detail_field(
            rows,
            &credential_id,
            tr(locale, "security.field_parse_error"),
            parse_error,
        );
    } else if inspection.is_some() {
        push_credential_detail_field(
            rows,
            &credential_id,
            tr(locale, "security.field_parse_status"),
            tr(locale, "security.value_ok").to_owned(),
        );
    }

    match (&credential.kind, inspection) {
        (CredentialKind::PrivateKey, Some(inspection)) => {
            push_optional_credential_detail_field(
                rows,
                &credential_id,
                tr(locale, "security.field_encrypted"),
                inspection.encrypted.map(|encrypted| {
                    tr(
                        locale,
                        if encrypted {
                            "security.value_yes"
                        } else {
                            "security.value_no"
                        },
                    )
                    .to_owned()
                }),
            );
            push_optional_credential_detail_field(
                rows,
                &credential_id,
                tr(locale, "security.field_comment"),
                inspection.comment.clone(),
            );
            push_optional_credential_detail_field(
                rows,
                &credential_id,
                tr(locale, "security.field_public_key"),
                inspection.public_key.clone(),
            );
        }
        (CredentialKind::Password, Some(inspection)) => {
            push_optional_credential_detail_field(
                rows,
                &credential_id,
                tr(locale, "security.field_password_length"),
                inspection.password_length.map(|length| length.to_string()),
            );
        }
        (CredentialKind::Certificate, Some(inspection)) => {
            if let Some(certificate) = inspection.certificate.as_ref() {
                push_optional_credential_detail_field(
                    rows,
                    &credential_id,
                    tr(locale, "security.certificate_type_label"),
                    certificate.cert_type.clone(),
                );
                push_optional_credential_detail_field(
                    rows,
                    &credential_id,
                    tr(locale, "security.certificate_serial_label"),
                    certificate.serial.map(|serial| serial.to_string()),
                );
                push_optional_credential_detail_field(
                    rows,
                    &credential_id,
                    tr(locale, "security.certificate_key_id_label"),
                    certificate.key_id.clone(),
                );
                if !certificate.principals.is_empty() {
                    push_credential_detail_field(
                        rows,
                        &credential_id,
                        tr(locale, "security.certificate_principals_label"),
                        certificate.principals.join(", "),
                    );
                }
                push_optional_credential_detail_field(
                    rows,
                    &credential_id,
                    tr(locale, "security.field_valid_after"),
                    certificate
                        .valid_after_unix_secs
                        .map(|seconds| seconds.to_string()),
                );
                push_optional_credential_detail_field(
                    rows,
                    &credential_id,
                    tr(locale, "security.field_valid_before"),
                    certificate
                        .valid_before_unix_secs
                        .map(|seconds| seconds.to_string()),
                );
                push_optional_credential_detail_field(
                    rows,
                    &credential_id,
                    tr(locale, "security.field_ca_fingerprint"),
                    certificate.ca_fingerprint.clone(),
                );
                push_optional_credential_detail_field(
                    rows,
                    &credential_id,
                    tr(locale, "security.field_subject_fingerprint"),
                    certificate.subject_fingerprint.clone(),
                );
                push_optional_credential_detail_field(
                    rows,
                    &credential_id,
                    tr(locale, "security.field_critical_options"),
                    certificate.critical_options_json.clone(),
                );
                push_optional_credential_detail_field(
                    rows,
                    &credential_id,
                    tr(locale, "security.field_extensions"),
                    certificate.extensions_json.clone(),
                );
            }
        }
        _ => {}
    }
}

fn push_optional_credential_detail_field(
    rows: &mut Vec<CredentialDetailFieldViewModel>,
    credential_id: &str,
    label: &str,
    value: Option<String>,
) {
    if let Some(value) = value.filter(|value| !value.is_empty()) {
        push_credential_detail_field(rows, credential_id, label, value);
    }
}

fn push_credential_detail_field(
    rows: &mut Vec<CredentialDetailFieldViewModel>,
    credential_id: &str,
    label: &str,
    value: String,
) {
    let field_index = rows
        .iter()
        .filter(|field| field.credential_id == credential_id)
        .count() as i32;
    rows.push(CredentialDetailFieldViewModel {
        credential_id: credential_id.to_owned(),
        label: label.to_owned(),
        value,
        row: field_index / 2,
        col: field_index % 2,
    });
}
