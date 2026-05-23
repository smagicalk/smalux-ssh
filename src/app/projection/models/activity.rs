//! 活动列表 Slint 模型转换。

use slint::{ModelRc, VecModel};

use crate::app::ActivityRow;
use crate::app::view_model::ActivityViewModel;

pub(in crate::app::projection) fn activity_model(
    items: &[ActivityViewModel],
) -> ModelRc<ActivityRow> {
    let rows = items
        .iter()
        .map(|item| ActivityRow {
            label: item.label.into(),
            value: item.value.as_str().into(),
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}
