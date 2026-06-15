//! 凭据左侧树形列表展示模型。

use crate::app::state::AsDesktopStateView;
use crate::model::{
    CredentialGroup, CredentialGroupId, CredentialKind, CredentialMetadata, SecretRecord,
};

use super::i18n::{Locale, locale_for_state, tr};
use super::tools_credentials_common::{
    credential_count_label, credential_group_label, credential_group_path, credential_icon_key,
    credential_kind_key, credential_kind_label, credential_matches, credential_row_id,
    credential_secret_available, credential_storage_label, credential_visible_in_security,
    key_algorithm_key, key_algorithm_label,
};
use super::tools_types::CredentialRowViewModel;

const CREDENTIAL_TREE_GUIDE_LEVELS: usize = 8;
type CredentialTreeGuides = [bool; CREDENTIAL_TREE_GUIDE_LEVELS];

pub(in crate::app::view_model) fn credential_rows(
    state: impl AsDesktopStateView,
) -> Vec<CredentialRowViewModel> {
    let state = state.as_desktop_state_view();
    let locale = locale_for_state(state);
    let empty = tr(locale, "tool.empty_value");
    let query = state
        .ui
        .workspace
        .credential_search_query
        .trim()
        .to_lowercase();
    let search_active = !query.is_empty();
    let collapsed_nodes = &state.ui.workspace.collapsed_credential_tree_nodes;
    let root_count = if query.is_empty() {
        state
            .storage
            .credentials
            .iter()
            .filter(|credential| credential_visible_in_security(&credential.kind))
            .count()
    } else {
        state
            .storage
            .credentials
            .iter()
            .filter(|credential| credential_visible_in_security(&credential.kind))
            .filter(|credential| credential_matches(credential, &query, locale))
            .count()
    };
    let mut rows = Vec::new();
    let root_id = "group:all";
    let root_collapsed = !search_active && credential_node_collapsed(collapsed_nodes, root_id);

    rows.push(CredentialRowViewModel {
        id: root_id.to_owned(),
        name: tr(locale, "security.root").to_owned(),
        group_id: String::new(),
        group_path: tr(locale, "security.root").to_owned(),
        kind_key: "Root",
        kind: tr(locale, "security.field_group").to_owned(),
        username: String::new(),
        secret_ref: String::new(),
        secret_available: false,
        algorithm: String::new(),
        algorithm_key: String::new(),
        fingerprint: String::new(),
        meta: credential_count_label(root_count, locale),
        icon_key: "folder",
        depth: 0,
        node_kind: "Group",
        accent_index: 0,
        expandable: true,
        expanded: !root_collapsed,
        has_next_sibling: false,
        guide_0: false,
        guide_1: false,
        guide_2: false,
        guide_3: false,
        guide_4: false,
        guide_5: false,
        guide_6: false,
        guide_7: false,
    });

    if root_collapsed {
        return rows;
    }

    for (kind, accent_index) in [
        (CredentialKind::PrivateKey, 0),
        (CredentialKind::Certificate, 1),
        (CredentialKind::Password, 2),
    ] {
        let kind_node_id = format!("group:{}", credential_kind_key(&kind));
        let kind_collapsed =
            !search_active && credential_node_collapsed(collapsed_nodes, &kind_node_id);
        let group_label = credential_group_label(&kind, locale);
        let group_matches = !query.is_empty() && group_label.to_lowercase().contains(&query);
        let group_credentials = state
            .storage
            .credentials
            .iter()
            .filter(|credential| credential.kind == kind)
            .filter(|credential| credential.group_id.is_none())
            .filter(|credential| {
                query.is_empty() || group_matches || credential_matches(credential, &query, locale)
            })
            .collect::<Vec<_>>();
        let mut custom_group_rows = Vec::new();
        let custom_group_visible = append_credential_group_rows(
            &mut custom_group_rows,
            &state.storage.credential_groups,
            &state.storage.credentials,
            &kind,
            None,
            2,
            &query,
            &state.storage.secrets,
            collapsed_nodes,
            search_active,
            locale,
            empty,
        );
        let direct_custom_group_count = state
            .storage
            .credential_groups
            .iter()
            .filter(|group| group.kind == kind && group.parent_id.is_none())
            .count();
        let expandable = direct_custom_group_count + group_credentials.len() > 0;

        if !query.is_empty()
            && !group_matches
            && group_credentials.is_empty()
            && !custom_group_visible
        {
            continue;
        }

        rows.push(CredentialRowViewModel {
            id: kind_node_id,
            name: group_label.to_owned(),
            group_id: String::new(),
            group_path: group_label.to_owned(),
            kind_key: credential_kind_key(&kind),
            kind: tr(locale, "security.field_group").to_owned(),
            username: String::new(),
            secret_ref: String::new(),
            secret_available: false,
            algorithm: String::new(),
            algorithm_key: String::new(),
            fingerprint: String::new(),
            meta: credential_count_label(group_credentials.len(), locale),
            icon_key: "folder",
            depth: 1,
            node_kind: "Group",
            accent_index,
            expandable,
            expanded: !kind_collapsed,
            has_next_sibling: false,
            guide_0: false,
            guide_1: false,
            guide_2: false,
            guide_3: false,
            guide_4: false,
            guide_5: false,
            guide_6: false,
            guide_7: false,
        });

        if !kind_collapsed {
            rows.extend(custom_group_rows);

            for credential in group_credentials {
                rows.push(credential_row(
                    credential,
                    2,
                    accent_index,
                    &state.storage.credential_groups,
                    &state.storage.secrets,
                    locale,
                    empty,
                ));
            }
        }
    }

    apply_credential_tree_guides(&mut rows);
    rows
}

fn append_credential_group_rows(
    rows: &mut Vec<CredentialRowViewModel>,
    groups: &[CredentialGroup],
    credentials: &[CredentialMetadata],
    kind: &CredentialKind,
    parent_id: Option<CredentialGroupId>,
    depth: i32,
    query: &str,
    secrets: &[SecretRecord],
    collapsed_nodes: &[String],
    search_active: bool,
    locale: Locale,
    empty: &str,
) -> bool {
    let mut any_visible = false;
    let mut siblings = groups
        .iter()
        .filter(|group| group.kind == *kind && group.parent_id == parent_id)
        .collect::<Vec<_>>();
    siblings.sort_by(|left, right| {
        left.sort_order
            .cmp(&right.sort_order)
            .then_with(|| left.name.cmp(&right.name))
    });

    for group in siblings {
        let mut child_rows = Vec::new();
        let child_visible = append_credential_group_rows(
            &mut child_rows,
            groups,
            credentials,
            kind,
            Some(group.id),
            depth + 1,
            query,
            secrets,
            collapsed_nodes,
            search_active,
            locale,
            empty,
        );
        let group_credentials = credentials
            .iter()
            .filter(|credential| credential.kind == *kind)
            .filter(|credential| credential.group_id == Some(group.id))
            .filter(|credential| query.is_empty() || credential_matches(credential, query, locale))
            .collect::<Vec<_>>();
        let group_matches = query.is_empty()
            || group.name.to_lowercase().contains(query)
            || tr(locale, "security.field_group")
                .to_lowercase()
                .contains(query);

        if query.is_empty() || group_matches || child_visible || !group_credentials.is_empty() {
            let node_id = format!("credential-group:{}", group.id.0);
            let collapsed = !search_active && credential_node_collapsed(collapsed_nodes, &node_id);
            let direct_child_count = groups
                .iter()
                .filter(|child| child.kind == *kind && child.parent_id == Some(group.id))
                .count();
            let expandable = direct_child_count + group_credentials.len() > 0;
            rows.push(CredentialRowViewModel {
                id: node_id,
                name: group.name.clone(),
                group_id: format!("credential-group:{}", group.id.0),
                group_path: credential_group_path(groups, Some(group.id), &group.kind, locale),
                kind_key: credential_kind_key(&group.kind),
                kind: tr(locale, "security.field_group").to_owned(),
                username: String::new(),
                secret_ref: String::new(),
                secret_available: false,
                algorithm: String::new(),
                algorithm_key: String::new(),
                fingerprint: String::new(),
                meta: credential_count_label(direct_child_count, locale),
                icon_key: "folder",
                depth,
                node_kind: "CredentialGroup",
                accent_index: 4,
                expandable,
                expanded: !collapsed,
                has_next_sibling: false,
                guide_0: false,
                guide_1: false,
                guide_2: false,
                guide_3: false,
                guide_4: false,
                guide_5: false,
                guide_6: false,
                guide_7: false,
            });

            if !collapsed {
                rows.extend(child_rows);
                for credential in group_credentials {
                    rows.push(credential_row(
                        credential,
                        depth + 1,
                        4,
                        groups,
                        secrets,
                        locale,
                        empty,
                    ));
                }
            }
            any_visible = true;
        }
    }

    any_visible
}

fn credential_row(
    credential: &CredentialMetadata,
    depth: i32,
    accent_index: i32,
    groups: &[CredentialGroup],
    secrets: &[SecretRecord],
    locale: Locale,
    empty: &str,
) -> CredentialRowViewModel {
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
    let secret_available = credential_secret_available(credential, secrets);
    let secret_ref = credential_storage_label(credential, secret_available, locale, empty);
    let username = credential
        .username
        .clone()
        .unwrap_or_else(|| empty.to_owned());

    CredentialRowViewModel {
        id: credential_row_id(credential),
        name: credential.name.clone(),
        group_id: credential
            .group_id
            .map(|id| format!("credential-group:{}", id.0))
            .unwrap_or_default(),
        group_path: credential_group_path(groups, credential.group_id, &credential.kind, locale),
        kind_key: credential_kind_key(&credential.kind),
        kind: credential_kind_label(&credential.kind, locale).to_owned(),
        username,
        secret_ref,
        secret_available,
        algorithm,
        algorithm_key,
        fingerprint,
        meta: credential
            .fingerprint
            .clone()
            .or_else(|| credential.key_algorithm.as_ref().map(key_algorithm_label))
            .or_else(|| credential.username.clone())
            .unwrap_or_else(|| credential_kind_label(&credential.kind, locale).to_owned()),
        icon_key: credential_icon_key(&credential.kind),
        depth,
        node_kind: "Credential",
        accent_index,
        expandable: false,
        expanded: false,
        has_next_sibling: false,
        guide_0: false,
        guide_1: false,
        guide_2: false,
        guide_3: false,
        guide_4: false,
        guide_5: false,
        guide_6: false,
        guide_7: false,
    }
}

fn apply_credential_tree_guides(rows: &mut [CredentialRowViewModel]) {
    for index in 0..rows.len() {
        let depth = rows[index].depth;
        rows[index].has_next_sibling = rows[index + 1..]
            .iter()
            .take_while(|row| row.depth >= depth)
            .any(|row| row.depth == depth);
    }

    let mut active_guides = empty_credential_tree_guides();
    for row in rows {
        let guide_0 = active_guides[0] && row.depth > 1;
        let guide_1 = active_guides[1] && row.depth > 2;
        let guide_2 = active_guides[2] && row.depth > 3;
        let guide_3 = active_guides[3] && row.depth > 4;
        let guide_4 = active_guides[4] && row.depth > 5;
        let guide_5 = active_guides[5] && row.depth > 6;
        let guide_6 = active_guides[6] && row.depth > 7;
        let guide_7 = active_guides[7] && row.depth > 8;

        row.guide_0 = guide_0;
        row.guide_1 = guide_1;
        row.guide_2 = guide_2;
        row.guide_3 = guide_3;
        row.guide_4 = guide_4;
        row.guide_5 = guide_5;
        row.guide_6 = guide_6;
        row.guide_7 = guide_7;

        if let Ok(depth) = usize::try_from(row.depth.saturating_sub(1)) {
            if row.depth > 0 && depth < CREDENTIAL_TREE_GUIDE_LEVELS {
                active_guides[depth] = row.has_next_sibling;
                active_guides
                    .iter_mut()
                    .skip(depth + 1)
                    .for_each(|guide| *guide = false);
            }
        }
    }
}

fn empty_credential_tree_guides() -> CredentialTreeGuides {
    [false; CREDENTIAL_TREE_GUIDE_LEVELS]
}

fn credential_node_collapsed(collapsed_nodes: &[String], node_id: &str) -> bool {
    collapsed_nodes.iter().any(|collapsed| collapsed == node_id)
}
