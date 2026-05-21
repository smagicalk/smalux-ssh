//! 启动流程共享时间函数。

use std::time::{SystemTime, UNIX_EPOCH};

pub(in crate::model::app_state) fn unix_now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
