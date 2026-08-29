//! Debug 日志面板与全局 Tracing 日志同步。

use std::rc::Rc;
use crate::generated::{AppWindow, LogEntryData};


/// 同步全局 Tracing 实时事件日志到 Slint UI 调试抽屉。
pub(crate) fn sync_ui_debug_logs(w: &AppWindow) {
    if let Ok(buf) = smagical_debug::get_global_log_buffer().lock() {
        let entries = buf.get_all();
        let slint_entries: Vec<LogEntryData> = entries
            .into_iter()
            .map(|e| LogEntryData {
                timestamp: e.timestamp.into(),
                level: e.level.into(),
                module: e.module.into(),
                message: e.message.into(),
            })
            .collect();
        w.set_debug_logs(slint::ModelRc::from(Rc::new(slint::VecModel::from(slint_entries))));
    }
}
