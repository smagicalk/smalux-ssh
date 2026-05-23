//! 通用 Slint 模型转换。

use slint::{ModelRc, SharedString, VecModel};

pub(in crate::app::projection) fn string_model(items: &[String]) -> ModelRc<SharedString> {
    let rows = items
        .iter()
        .map(|item| SharedString::from(item.as_str()))
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}
