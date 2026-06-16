//! UI view model 层。

use smagical_core::CoreState;

/// 首页展示模型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeViewModel {
    pub host_summary: String,
}

impl HomeViewModel {
    pub fn from_core(core: &CoreState) -> Self {
        let host_summary = core
            .hosts
            .first()
            .map(|host| format!("{} ({}:{})", host.name, host.address, host.port))
            .unwrap_or_else(|| "暂无主机".to_owned());

        Self { host_summary }
    }
}
