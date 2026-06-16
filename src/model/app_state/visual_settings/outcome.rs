//! 视觉配置消息结果构造。

use crate::model::HostId;

use super::super::AppUpdateOutcome;

#[cfg_attr(not(feature = "desktop"), allow(dead_code))]
pub(super) fn missing_host(host_id: HostId) -> AppUpdateOutcome {
    AppUpdateOutcome {
        error: Some(format!("找不到主机：{}", host_id.0)),
        ..AppUpdateOutcome::default()
    }
}
