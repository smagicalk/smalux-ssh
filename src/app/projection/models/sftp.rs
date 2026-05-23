//! SFTP 列表 Slint 模型转换。

use slint::{ModelRc, VecModel};

use crate::app::SftpEntryRow;
use crate::app::view_model::SftpEntryViewModel;

pub(in crate::app::projection) fn sftp_entry_model(
    items: &[SftpEntryViewModel],
) -> ModelRc<SftpEntryRow> {
    let rows = items
        .iter()
        .map(|item| SftpEntryRow {
            name: item.name.as_str().into(),
            path: item.path.as_str().into(),
            kind: item.kind.into(),
            size: item.size.as_str().into(),
            selected: item.selected,
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}
