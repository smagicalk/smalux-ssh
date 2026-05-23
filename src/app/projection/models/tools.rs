//! 工具列表 Slint 模型转换。

use slint::{ModelRc, VecModel};

use crate::app::ToolItemRow;
use crate::app::view_model::ToolItemViewModel;

pub(in crate::app::projection) fn tool_item_model(
    items: &[ToolItemViewModel],
) -> ModelRc<ToolItemRow> {
    let rows = items
        .iter()
        .map(|item| ToolItemRow {
            title: item.title.as_str().into(),
            subtitle: item.subtitle.as_str().into(),
            meta: item.meta.as_str().into(),
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}
