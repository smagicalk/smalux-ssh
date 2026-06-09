//! 片段工具 Slint 模型转换。

use slint::{ModelRc, VecModel};

use crate::app::SnippetRow;
use crate::app::view_model::SnippetRowViewModel;

pub(in crate::app::projection) fn snippet_row_model(
    items: &[SnippetRowViewModel],
) -> ModelRc<SnippetRow> {
    let rows = items
        .iter()
        .map(|item| SnippetRow {
            id: item.id.as_str().into(),
            parent_id: item.parent_id.as_str().into(),
            name: item.name.as_str().into(),
            description: item.description.as_str().into(),
            command_template: item.command_template.as_str().into(),
            scope: item.scope.as_str().into(),
            scope_key: item.scope_key.into(),
            variables: item.variables.as_str().into(),
            variable_names: item.variable_names.as_str().into(),
            arguments: item.arguments.as_str().into(),
            argument_values: item.argument_values.as_str().into(),
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
