//! 凭据工具 Slint 模型转换。

use slint::{ModelRc, VecModel};

use crate::app::view_model::{
    CredentialDetailFieldViewModel, CredentialGroupContentViewModel, CredentialRowViewModel,
};
use crate::app::{CredentialDetailFieldRow, CredentialGroupContentRow, CredentialRow};

pub(in crate::app::projection) fn credential_row_model(
    items: &[CredentialRowViewModel],
) -> ModelRc<CredentialRow> {
    let rows = items
        .iter()
        .map(|item| CredentialRow {
            id: item.id.as_str().into(),
            name: item.name.as_str().into(),
            group_id: item.group_id.as_str().into(),
            group_path: item.group_path.as_str().into(),
            kind_key: item.kind_key.into(),
            kind: item.kind.as_str().into(),
            username: item.username.as_str().into(),
            secret_ref: item.secret_ref.as_str().into(),
            secret_available: item.secret_available,
            algorithm: item.algorithm.as_str().into(),
            algorithm_key: item.algorithm_key.as_str().into(),
            fingerprint: item.fingerprint.as_str().into(),
            meta: item.meta.as_str().into(),
            icon_key: item.icon_key.into(),
            depth: item.depth,
            node_kind: item.node_kind.into(),
            accent_index: item.accent_index,
            expandable: item.expandable,
            expanded: item.expanded,
            has_next_sibling: item.has_next_sibling,
            guide_0: item.guide_0,
            guide_1: item.guide_1,
            guide_2: item.guide_2,
            guide_3: item.guide_3,
            guide_4: item.guide_4,
            guide_5: item.guide_5,
            guide_6: item.guide_6,
            guide_7: item.guide_7,
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

pub(in crate::app::projection) fn credential_group_content_model(
    items: &[CredentialGroupContentViewModel],
) -> ModelRc<CredentialGroupContentRow> {
    let rows = items
        .iter()
        .map(|item| CredentialGroupContentRow {
            parent_id: item.parent_id.as_str().into(),
            id: item.id.as_str().into(),
            name: item.name.as_str().into(),
            group_id: item.group_id.as_str().into(),
            group_path: item.group_path.as_str().into(),
            kind_key: item.kind_key.into(),
            kind: item.kind.as_str().into(),
            node_kind: item.node_kind.into(),
            username: item.username.as_str().into(),
            secret_ref: item.secret_ref.as_str().into(),
            secret_available: item.secret_available,
            algorithm: item.algorithm.as_str().into(),
            algorithm_key: item.algorithm_key.as_str().into(),
            fingerprint: item.fingerprint.as_str().into(),
            detail: item.detail.as_str().into(),
            meta: item.meta.as_str().into(),
            icon_key: item.icon_key.into(),
            accent_index: item.accent_index,
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}

pub(in crate::app::projection) fn credential_detail_field_model(
    items: &[CredentialDetailFieldViewModel],
) -> ModelRc<CredentialDetailFieldRow> {
    let rows = items
        .iter()
        .map(|item| CredentialDetailFieldRow {
            credential_id: item.credential_id.as_str().into(),
            label: item.label.as_str().into(),
            value: item.value.as_str().into(),
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}
