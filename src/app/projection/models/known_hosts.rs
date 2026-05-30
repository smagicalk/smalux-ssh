//! Known hosts Slint 模型转换。

use slint::{ModelRc, VecModel};

use crate::app::KnownHostRow;
use crate::app::view_model::KnownHostViewModel;

pub(in crate::app::projection) fn known_host_model(
    items: &[KnownHostViewModel],
) -> ModelRc<KnownHostRow> {
    let rows = items
        .iter()
        .map(|item| KnownHostRow {
            host: item.host.as_str().into(),
            port: i32::from(item.port),
            fingerprint: item.fingerprint.as_str().into(),
            status_key: item.status_key.as_str().into(),
            status: item.status.as_str().into(),
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}
