//! Debug 日志面板与全局 Tracing 日志同步。
//!
//! 将内存 RingBuffer 中的实时结构化诊断日志映射为 Slint 数据模型并推送到前端界面。

use std::rc::Rc;
use crate::generated::{AppWindow, LogEntryData};

/// 同步全局 Tracing 实时事件日志到 Slint UI 调试抽屉。
///
/// 从 `smagical_debug` 的全局互斥环形缓冲区中提取所有日志条目，将其转化为 Slint 的 `LogEntryData` 向量并刷新 UI。
///
/// # 参数
/// - `w`: Slint 主窗口句柄引用
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

