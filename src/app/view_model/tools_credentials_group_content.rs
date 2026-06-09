//! 凭据分组内容展示模型。

use crate::model::{AppState, CredentialGroup, CredentialKind, CredentialMetadata, SecretRecord};

use super::i18n::{Locale, locale_for_state, tr};
use super::tools_credentials_common::{
    credential_count_label, credential_group_label, credential_group_path, credential_icon_key,
    credential_kind_key, credential_kind_label, credential_row_id, credential_secret_available,
    credential_storage_label, key_algorithm_key, key_algorithm_label,
};
use super::tools_types::CredentialGroupContentViewModel;

pub(in crate::app::view_model) fn credential_group_contents(
    state: &AppState,
) -> Vec<CredentialGroupContentViewModel> {
    let locale = locale_for_state(state);
    let empty = tr(locale, "tool.empty_value");
    let mut rows = Vec::new();

    rows.extend(credential_group_content_for_root(state, locale));
    for kind in [
        CredentialKind::PrivateKey,
        CredentialKind::Certificate,
        CredentialKind::Password,
    ] {
        rows.extend(credential_group_content_for_kind(
            state, &kind, locale, empty,
        ));
        rows.extend(credential_group_content_for_custom_groups(
            state, &kind, locale, empty,
        ));
    }

    rows
}

fn credential_group_content_for_root(
    state: &AppState,
    locale: Locale,
) -> Vec<CredentialGroupContentViewModel> {
    [
        (CredentialKind::PrivateKey, 0),
        (CredentialKind::Certificate, 1),
        (CredentialKind::Password, 2),
    ]
    .into_iter()
    .map(|(kind, accent_index)| {
        let count = state
            .storage
            .credentials
            .iter()
            .filter(|credential| credential.kind == kind)
            .count();
        CredentialGroupContentViewModel {
            parent_id: "group:all".to_owned(),
            id: format!("group:{}", credential_kind_key(&kind)),
            name: credential_group_label(&kind, locale).to_owned(),
            group_id: String::new(),
            group_path: credential_group_label(&kind, locale).to_owned(),
            kind_key: credential_kind_key(&kind),
            kind: tr(locale, "security.field_group").to_owned(),
            node_kind: "Group",
            username: String::new(),
            secret_ref: String::new(),
            secret_available: false,
            algorithm: String::new(),
            algorithm_key: String::new(),
            fingerprint: String::new(),
            detail: tr(locale, "security.group_content_category").to_owned(),
            meta: credential_count_label(count, locale),
            icon_key: credential_icon_key(&kind),
            accent_index,
        }
    })
    .collect()
}

fn credential_group_content_for_kind(
    state: &AppState,
    kind: &CredentialKind,
    locale: Locale,
    empty: &str,
) -> Vec<CredentialGroupContentViewModel> {
    let parent_id = format!("group:{}", credential_kind_key(kind));
    let mut groups = state
        .storage
        .credential_groups
        .iter()
        .filter(|group| group.kind == *kind && group.parent_id.is_none())
        .collect::<Vec<_>>();
    groups.sort_by(|left, right| {
        left.sort_order
            .cmp(&right.sort_order)
            .then_with(|| left.name.cmp(&right.name))
    });
    let mut credentials = state
        .storage
        .credentials
        .iter()
        .filter(|credential| credential.kind == *kind && credential.group_id.is_none())
        .collect::<Vec<_>>();
    credentials.sort_by(|left, right| left.name.cmp(&right.name));

    let mut rows = groups
        .into_iter()
        .map(|group| {
            credential_group_content_group_row(
                &parent_id,
                group,
                &state.storage.credential_groups,
                locale,
            )
        })
        .collect::<Vec<_>>();
    rows.extend(credentials.into_iter().map(|credential| {
        credential_group_content_credential_row(
            &parent_id,
            credential,
            &state.storage.credential_groups,
            &state.storage.secrets,
            locale,
            empty,
        )
    }));
    if rows.is_empty() {
        rows.push(credential_group_content_empty_row(&parent_id, locale));
    }
    rows
}

fn credential_group_content_for_custom_groups(
    state: &AppState,
    kind: &CredentialKind,
    locale: Locale,
    empty: &str,
) -> Vec<CredentialGroupContentViewModel> {
    let mut rows = Vec::new();
    for parent in state
        .storage
        .credential_groups
        .iter()
        .filter(|group| group.kind == *kind)
    {
        let parent_id = format!("credential-group:{}", parent.id.0);
        let start_len = rows.len();
        let mut child_groups = state
            .storage
            .credential_groups
            .iter()
            .filter(|group| group.kind == *kind && group.parent_id == Some(parent.id))
            .collect::<Vec<_>>();
        child_groups.sort_by(|left, right| {
            left.sort_order
                .cmp(&right.sort_order)
                .then_with(|| left.name.cmp(&right.name))
        });
        rows.extend(child_groups.into_iter().map(|group| {
            credential_group_content_group_row(
                &parent_id,
                group,
                &state.storage.credential_groups,
                locale,
            )
        }));

        let mut credentials = state
            .storage
            .credentials
            .iter()
            .filter(|credential| credential.kind == *kind)
            .filter(|credential| credential.group_id == Some(parent.id))
            .collect::<Vec<_>>();
        credentials.sort_by(|left, right| left.name.cmp(&right.name));
        rows.extend(credentials.into_iter().map(|credential| {
            credential_group_content_credential_row(
                &parent_id,
                credential,
                &state.storage.credential_groups,
                &state.storage.secrets,
                locale,
                empty,
            )
        }));
        if rows.len() == start_len {
            rows.push(credential_group_content_empty_row(&parent_id, locale));
        }
    }
    rows
}

fn credential_group_content_group_row(
    parent_id: &str,
    group: &CredentialGroup,
    groups: &[CredentialGroup],
    locale: Locale,
) -> CredentialGroupContentViewModel {
    let group_path = credential_group_path(groups, Some(group.id), &group.kind, locale);
    CredentialGroupContentViewModel {
        parent_id: parent_id.to_owned(),
        id: format!("credential-group:{}", group.id.0),
        name: group.name.clone(),
        group_id: format!("credential-group:{}", group.id.0),
        group_path: group_path.clone(),
        kind_key: credential_kind_key(&group.kind),
        kind: tr(locale, "security.field_group").to_owned(),
        node_kind: "CredentialGroup",
        username: String::new(),
        secret_ref: String::new(),
        secret_available: false,
        algorithm: String::new(),
        algorithm_key: String::new(),
        fingerprint: String::new(),
        detail: group_path,
        meta: tr(locale, "security.group_content_folder").to_owned(),
        icon_key: "folder",
        accent_index: 4,
    }
}

fn credential_group_content_credential_row(
    parent_id: &str,
    credential: &CredentialMetadata,
    groups: &[CredentialGroup],
    secrets: &[SecretRecord],
    locale: Locale,
    empty: &str,
) -> CredentialGroupContentViewModel {
    let secret_available = credential_secret_available(credential, secrets);
    let secret_ref = credential_storage_label(credential, secret_available, locale, empty);
    let username = credential
        .username
        .clone()
        .unwrap_or_else(|| empty.to_owned());
    let algorithm = credential
        .key_algorithm
        .as_ref()
        .map(key_algorithm_label)
        .unwrap_or_else(|| empty.to_owned());
    let algorithm_key = credential
        .key_algorithm
        .as_ref()
        .map(key_algorithm_key)
        .unwrap_or_default();
    let fingerprint = credential
        .fingerprint
        .clone()
        .unwrap_or_else(|| empty.to_owned());

    CredentialGroupContentViewModel {
        parent_id: parent_id.to_owned(),
        id: credential_row_id(credential),
        name: credential.name.clone(),
        group_id: credential
            .group_id
            .map(|id| format!("credential-group:{}", id.0))
            .unwrap_or_default(),
        group_path: credential_group_path(groups, credential.group_id, &credential.kind, locale),
        kind_key: credential_kind_key(&credential.kind),
        kind: credential_kind_label(&credential.kind, locale).to_owned(),
        node_kind: "Credential",
        username,
        secret_ref,
        secret_available,
        algorithm,
        algorithm_key,
        fingerprint,
        detail: credential
            .username
            .clone()
            .unwrap_or_else(|| credential_kind_label(&credential.kind, locale).to_owned()),
        meta: credential
            .fingerprint
            .clone()
            .or_else(|| credential.key_algorithm.as_ref().map(key_algorithm_label))
            .unwrap_or_else(|| empty.to_owned()),
        icon_key: credential_icon_key(&credential.kind),
        accent_index: 4,
    }
}

fn credential_group_content_empty_row(
    parent_id: &str,
    locale: Locale,
) -> CredentialGroupContentViewModel {
    CredentialGroupContentViewModel {
        parent_id: parent_id.to_owned(),
        id: format!("empty:{parent_id}"),
        name: tr(locale, "security.group_content_empty").to_owned(),
        group_id: String::new(),
        group_path: String::new(),
        kind_key: "",
        kind: String::new(),
        node_kind: "Empty",
        username: String::new(),
        secret_ref: String::new(),
        secret_available: false,
        algorithm: String::new(),
        algorithm_key: String::new(),
        fingerprint: String::new(),
        detail: tr(locale, "security.group_caption").to_owned(),
        meta: String::new(),
        icon_key: "cluster",
        accent_index: 0,
    }
}
