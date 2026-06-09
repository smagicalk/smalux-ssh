//! 片段工具页展示模型。

use crate::model::{AppState, Snippet, SnippetGroup, SnippetGroupId, SnippetScope};

use super::i18n::{locale_for_state, tr};
use super::tools_types::{SnippetRowViewModel, ToolItemViewModel};

const SNIPPET_TREE_GUIDE_LEVELS: usize = 8;
type SnippetTreeGuides = [bool; SNIPPET_TREE_GUIDE_LEVELS];

pub(in crate::app::view_model) fn snippet_items(state: &AppState) -> Vec<ToolItemViewModel> {
    let locale = locale_for_state(state);
    state
        .storage
        .snippets
        .iter()
        .map(|snippet| ToolItemViewModel {
            title: snippet.name.clone(),
            subtitle: snippet
                .description
                .clone()
                .unwrap_or_else(|| snippet.default_command_template().to_owned()),
            meta: format!(
                "{}{}",
                snippet.variables.len(),
                tr(locale, "tool.snippet_vars_suffix")
            ),
        })
        .collect()
}

pub(in crate::app::view_model) fn snippet_rows(state: &AppState) -> Vec<SnippetRowViewModel> {
    let locale = locale_for_state(state);
    let query = state
        .ui
        .workspace
        .snippet_search_query
        .trim()
        .to_lowercase();
    let search_active = !query.is_empty();
    let collapsed_nodes = &state.ui.workspace.collapsed_snippet_tree_nodes;
    let mut rows = Vec::new();
    let root_id = "snippet-folder:all";
    let root_collapsed = !search_active && snippet_node_collapsed(collapsed_nodes, root_id);
    let root_count = filtered_snippet_count(&state.storage.snippets, &query);

    rows.push(snippet_folder_row(
        root_id,
        "",
        tr(locale, "snippets.scope_all"),
        tr(locale, "snippets.scope_all"),
        "Root",
        0,
        0,
        root_count,
        true,
        !root_collapsed,
        locale,
    ));

    if root_collapsed {
        return rows;
    }

    append_snippet_group_rows(
        &mut rows,
        &state.storage.snippet_groups,
        &state.storage.snippets,
        None,
        1,
        &query,
        collapsed_nodes,
        search_active,
        state,
        locale,
    );
    append_snippet_rows_for_group(
        &mut rows,
        &state.storage.snippets,
        None,
        1,
        0,
        &query,
        state,
        locale,
    );

    apply_snippet_tree_guides(&mut rows);
    rows
}

pub(in crate::app::view_model) fn snippet_target_options(
    state: &AppState,
) -> Vec<SnippetRowViewModel> {
    let locale = locale_for_state(state);
    let mut rows = Vec::new();
    let mut snippets = state.storage.snippets.iter().collect::<Vec<_>>();
    snippets.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));

    for snippet in snippets {
        append_snippet_target_rows(&mut rows, snippet, 0, 0, "", locale);
    }

    rows
}

fn append_snippet_group_rows(
    rows: &mut Vec<SnippetRowViewModel>,
    groups: &[SnippetGroup],
    snippets: &[Snippet],
    parent_id: Option<SnippetGroupId>,
    depth: i32,
    query: &str,
    collapsed_nodes: &[String],
    search_active: bool,
    state: &AppState,
    locale: super::i18n::Locale,
) -> bool {
    let mut visible = false;
    let mut child_groups = groups
        .iter()
        .filter(|group| group.parent_id == parent_id)
        .collect::<Vec<_>>();
    child_groups.sort_by(|left, right| {
        left.sort_order
            .cmp(&right.sort_order)
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });

    for group in child_groups {
        let folder_id = snippet_group_folder_id(group.id);
        let group_matches = snippet_text_matches(group.name.as_str(), query);
        let child_snippet_count = filtered_snippet_count_for_group(snippets, Some(group.id), query);
        let has_matching_descendant =
            snippet_group_has_matching_descendant(groups, snippets, group.id, query);

        if search_active && !group_matches && child_snippet_count == 0 && !has_matching_descendant {
            continue;
        }

        let collapsed = !search_active && snippet_node_collapsed(collapsed_nodes, &folder_id);
        let expandable =
            snippet_group_has_direct_children(groups, snippets, group.id) || search_active;
        rows.push(snippet_folder_row(
            folder_id.as_str(),
            snippet_parent_id(parent_id).as_str(),
            group.name.as_str(),
            tr(locale, "snippets.group_caption"),
            "Group",
            depth,
            snippet_group_accent_index(group.id),
            child_snippet_count + descendant_snippet_count(groups, snippets, group.id, query),
            expandable,
            !collapsed,
            locale,
        ));
        visible = true;

        if collapsed {
            continue;
        }

        let child_visible = append_snippet_group_rows(
            rows,
            groups,
            snippets,
            Some(group.id),
            depth + 1,
            query,
            collapsed_nodes,
            search_active,
            state,
            locale,
        );
        let snippet_visible = append_snippet_rows_for_group(
            rows,
            snippets,
            Some(group.id),
            depth + 1,
            snippet_group_accent_index(group.id),
            query,
            state,
            locale,
        );
        visible |= child_visible || snippet_visible;
    }
    visible
}

fn append_snippet_rows_for_group(
    rows: &mut Vec<SnippetRowViewModel>,
    snippets: &[Snippet],
    group_id: Option<SnippetGroupId>,
    depth: i32,
    accent_index: i32,
    query: &str,
    state: &AppState,
    locale: super::i18n::Locale,
) -> bool {
    let mut group_snippets = snippets
        .iter()
        .filter(|snippet| snippet.group_id == group_id)
        .filter(|snippet| snippet_matches(snippet, query))
        .collect::<Vec<_>>();
    group_snippets.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));

    for snippet in &group_snippets {
        let (scope, scope_key) = snippet_scope_label(snippet, state, locale);
        let snippet_id = snippet_row_id(snippet);
        let collapsed = query.is_empty()
            && snippet_node_collapsed(
                &state.ui.workspace.collapsed_snippet_tree_nodes,
                &snippet_id,
            );
        rows.push(snippet_row(
            snippet,
            snippet_parent_id(group_id),
            depth,
            accent_index,
            scope,
            scope_key,
            !collapsed,
            locale,
        ));
        if !collapsed {
            append_snippet_target_rows(rows, snippet, depth + 1, accent_index, query, locale);
        }
    }

    !group_snippets.is_empty()
}

fn snippet_folder_row(
    id: &str,
    parent_id: &str,
    name: &str,
    scope: &str,
    scope_key: &'static str,
    depth: i32,
    accent_index: i32,
    child_count: usize,
    expandable: bool,
    expanded: bool,
    locale: super::i18n::Locale,
) -> SnippetRowViewModel {
    SnippetRowViewModel {
        id: id.to_owned(),
        parent_id: parent_id.to_owned(),
        name: name.to_owned(),
        description: tr(locale, "snippets.group_caption").to_owned(),
        command_template: String::new(),
        scope: scope.to_owned(),
        scope_key,
        variables: String::new(),
        variable_names: String::new(),
        arguments: String::new(),
        argument_values: String::new(),
        meta: snippet_count_label(child_count, locale),
        icon_key: "folder",
        depth,
        node_kind: "Group",
        accent_index,
        expandable,
        expanded,
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

fn snippet_row(
    snippet: &Snippet,
    parent_id: String,
    depth: i32,
    accent_index: i32,
    scope: String,
    scope_key: &'static str,
    expanded: bool,
    locale: super::i18n::Locale,
) -> SnippetRowViewModel {
    let target_count = snippet.support_targets.len();
    SnippetRowViewModel {
        id: snippet_row_id(snippet),
        parent_id,
        name: snippet.name.clone(),
        description: snippet.description.clone().unwrap_or_default(),
        command_template: snippet.default_command_template().to_owned(),
        scope,
        scope_key,
        variables: snippet.variables.len().to_string(),
        variable_names: snippet_variable_names(snippet),
        arguments: snippet_default_arguments(snippet).len().to_string(),
        argument_values: snippet_argument_values(snippet),
        meta: snippet_target_count_label(target_count, locale),
        icon_key: "code",
        depth,
        node_kind: "Snippet",
        accent_index,
        expandable: target_count > 0,
        expanded,
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

fn append_snippet_target_rows(
    rows: &mut Vec<SnippetRowViewModel>,
    snippet: &Snippet,
    depth: i32,
    accent_index: i32,
    query: &str,
    locale: super::i18n::Locale,
) {
    let mut targets = snippet.support_targets.iter().collect::<Vec<_>>();
    targets.sort_by(|left, right| {
        left.sort_order.cmp(&right.sort_order).then_with(|| {
            left.display_name
                .to_lowercase()
                .cmp(&right.display_name.to_lowercase())
        })
    });

    for target in targets {
        let Some(implementation) = snippet
            .implementations
            .iter()
            .find(|implementation| implementation.id == target.implementation_id)
        else {
            continue;
        };
        if !query.is_empty()
            && !snippet_text_matches(target.display_name.as_str(), query)
            && !snippet_text_matches(target.target_key.as_str(), query)
            && !snippet_text_matches(implementation.command_template.as_str(), query)
        {
            continue;
        }
        rows.push(snippet_target_row(
            snippet,
            target,
            implementation,
            depth,
            accent_index,
            locale,
        ));
    }
}

fn snippet_target_row(
    snippet: &Snippet,
    target: &crate::model::SnippetSupportTarget,
    implementation: &crate::model::SnippetImplementation,
    depth: i32,
    accent_index: i32,
    locale: super::i18n::Locale,
) -> SnippetRowViewModel {
    let shared_count = snippet
        .support_targets
        .iter()
        .filter(|other| other.implementation_id == target.implementation_id)
        .count();
    SnippetRowViewModel {
        id: snippet_target_row_id(snippet, target),
        parent_id: snippet_row_id(snippet),
        name: snippet.name.clone(),
        description: implementation.name.clone(),
        command_template: implementation.command_template.clone(),
        scope: target.target_key.clone(),
        scope_key: if shared_count > 1 {
            "SupportTargetShared"
        } else {
            "SupportTarget"
        },
        variables: snippet.variables.len().to_string(),
        variable_names: snippet_variable_names(snippet),
        arguments: implementation.last_arguments.len().to_string(),
        argument_values: snippet_arguments_to_text(implementation.last_arguments.as_slice()),
        meta: if shared_count > 1 {
            format!(
                "{} {shared_count}",
                tr(locale, "snippets.shared_target_meta")
            )
        } else {
            implementation.shell.key().to_owned()
        },
        icon_key: "terminal",
        depth,
        node_kind: "SnippetTarget",
        accent_index: (accent_index + target.sort_order + 1) % 6,
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

fn snippet_row_id(snippet: &Snippet) -> String {
    format!("snippet:{}", snippet.id.0)
}

fn snippet_target_row_id(snippet: &Snippet, target: &crate::model::SnippetSupportTarget) -> String {
    format!("snippet-target:{}:{}", snippet.id.0, target.id.0)
}

fn snippet_group_folder_id(group_id: SnippetGroupId) -> String {
    format!("snippet-folder:group:{}", group_id.0)
}

fn snippet_parent_id(group_id: Option<SnippetGroupId>) -> String {
    group_id
        .map(snippet_group_folder_id)
        .unwrap_or_else(|| "snippet-folder:all".to_owned())
}

fn snippet_argument_values(snippet: &Snippet) -> String {
    snippet_arguments_to_text(snippet_default_arguments(snippet))
}

fn snippet_arguments_to_text(arguments: &[crate::model::SnippetArgument]) -> String {
    arguments
        .iter()
        .map(|argument| format!("{}={}", argument.name, argument.value))
        .collect::<Vec<_>>()
        .join("\n")
}

fn snippet_variable_names(snippet: &Snippet) -> String {
    snippet
        .variables
        .iter()
        .map(|variable| variable.name.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

fn filtered_snippet_count(snippets: &[Snippet], query: &str) -> usize {
    snippets
        .iter()
        .filter(|snippet| query.is_empty() || snippet_matches(snippet, query))
        .count()
}

fn filtered_snippet_count_for_group(
    snippets: &[Snippet],
    group_id: Option<SnippetGroupId>,
    query: &str,
) -> usize {
    snippets
        .iter()
        .filter(|snippet| snippet.group_id == group_id)
        .filter(|snippet| query.is_empty() || snippet_matches(snippet, query))
        .count()
}

fn snippet_group_has_direct_children(
    groups: &[SnippetGroup],
    snippets: &[Snippet],
    group_id: SnippetGroupId,
) -> bool {
    groups.iter().any(|group| group.parent_id == Some(group_id))
        || snippets
            .iter()
            .any(|snippet| snippet.group_id == Some(group_id))
}

fn snippet_group_has_matching_descendant(
    groups: &[SnippetGroup],
    snippets: &[Snippet],
    group_id: SnippetGroupId,
    query: &str,
) -> bool {
    descendant_snippet_count(groups, snippets, group_id, query) > 0
        || groups.iter().any(|group| {
            group.parent_id == Some(group_id)
                && (snippet_text_matches(group.name.as_str(), query)
                    || snippet_group_has_matching_descendant(groups, snippets, group.id, query))
        })
}

fn descendant_snippet_count(
    groups: &[SnippetGroup],
    snippets: &[Snippet],
    group_id: SnippetGroupId,
    query: &str,
) -> usize {
    groups
        .iter()
        .filter(|group| group.parent_id == Some(group_id))
        .map(|group| {
            filtered_snippet_count_for_group(snippets, Some(group.id), query)
                + descendant_snippet_count(groups, snippets, group.id, query)
        })
        .sum()
}

fn snippet_scope_label(
    snippet: &Snippet,
    state: &AppState,
    locale: super::i18n::Locale,
) -> (String, &'static str) {
    match snippet.scope {
        SnippetScope::Global => (tr(locale, "snippets.scope_global").to_owned(), "Global"),
        SnippetScope::Host(host_id) => {
            let label = state
                .storage
                .hosts
                .iter()
                .find(|host| host.id == host_id)
                .map(|host| host.name.clone())
                .unwrap_or_else(|| tr(locale, "snippets.scope_hosts").to_owned());
            (label, "Host")
        }
    }
}

fn snippet_group_accent_index(group_id: SnippetGroupId) -> i32 {
    (group_id.0.as_u128() % 6) as i32
}

fn snippet_matches(snippet: &Snippet, query: &str) -> bool {
    query.is_empty()
        || snippet_text_matches(snippet.name.as_str(), query)
        || snippet
            .description
            .as_deref()
            .is_some_and(|description| snippet_text_matches(description, query))
        || snippet.implementations.iter().any(|implementation| {
            snippet_text_matches(implementation.command_template.as_str(), query)
        })
        || snippet
            .variables
            .iter()
            .any(|variable| snippet_text_matches(variable.name.as_str(), query))
}

fn snippet_default_arguments(snippet: &Snippet) -> &[crate::model::SnippetArgument] {
    snippet
        .default_implementation()
        .map(|implementation| implementation.last_arguments.as_slice())
        .unwrap_or(&[])
}

fn snippet_text_matches(value: &str, query: &str) -> bool {
    query.is_empty() || value.to_lowercase().contains(query)
}

fn snippet_count_label(count: usize, locale: super::i18n::Locale) -> String {
    format!("{}{}", count, tr(locale, "snippets.count_suffix"))
}

fn snippet_target_count_label(count: usize, locale: super::i18n::Locale) -> String {
    format!("{}{}", count, tr(locale, "snippets.target_count_suffix"))
}

fn apply_snippet_tree_guides(rows: &mut [SnippetRowViewModel]) {
    for index in 0..rows.len() {
        let depth = rows[index].depth;
        rows[index].has_next_sibling = rows[index + 1..]
            .iter()
            .take_while(|row| row.depth >= depth)
            .any(|row| row.depth == depth);
    }

    let mut active_guides = empty_snippet_tree_guides();
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
            if row.depth > 0 && depth < SNIPPET_TREE_GUIDE_LEVELS {
                active_guides[depth] = row.has_next_sibling;
                active_guides
                    .iter_mut()
                    .skip(depth + 1)
                    .for_each(|guide| *guide = false);
            }
        }
    }
}

fn empty_snippet_tree_guides() -> SnippetTreeGuides {
    [false; SNIPPET_TREE_GUIDE_LEVELS]
}

fn snippet_node_collapsed(collapsed_nodes: &[String], node_id: &str) -> bool {
    collapsed_nodes.iter().any(|collapsed| collapsed == node_id)
}
