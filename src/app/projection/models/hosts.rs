//! 主机列表 Slint 模型转换。

use slint::{ModelRc, VecModel};

use crate::app::HostRow;
use crate::app::view_model::HostViewModel;

pub(in crate::app::projection) fn host_model(items: &[HostViewModel]) -> ModelRc<HostRow> {
    let rows = items
        .iter()
        .map(|host| HostRow {
            id: host.id.as_str().into(),
            name: host.name.as_str().into(),
            endpoint: host.endpoint.as_str().into(),
            auth: host.auth.into(),
            group: host.group.as_str().into(),
            tags: host.tags.as_str().into(),
            status: host.status.into(),
        })
        .collect::<Vec<_>>();
    ModelRc::new(VecModel::from(rows))
}
