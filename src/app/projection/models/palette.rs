//! 命令面板 Slint 模型转换。

use slint::{ModelRc, VecModel};

use crate::app::CommandPaletteRow;
use crate::app::view_model::CommandPaletteItemViewModel;

pub(in crate::app::projection) fn command_palette_model(
    items: &[CommandPaletteItemViewModel],
) -> ModelRc<CommandPaletteRow> {
    let rows = items
        .iter()
        .map(|item| CommandPaletteRow {
            id: item.id.as_str().into(),
            title: item.title.as_str().into(),
            subtitle: item.subtitle.as_str().into(),
            kind: item.kind.into(),
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}
