//! 标签页 Slint 模型转换。

use slint::{ModelRc, VecModel};

use crate::app::SessionTabRow;
use crate::app::view_model::SessionTabViewModel;

pub(in crate::app::projection) fn tab_model(
    items: &[SessionTabViewModel],
) -> ModelRc<SessionTabRow> {
    let rows = items
        .iter()
        .map(|tab| SessionTabRow {
            id: tab.id.as_str().into(),
            title: tab.title.as_str().into(),
            kind: tab.kind.into(),
            status_key: tab.status_key.into(),
            status: tab.status.into(),
            active: tab.active,
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}
