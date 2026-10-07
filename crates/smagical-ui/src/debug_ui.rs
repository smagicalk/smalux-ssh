//! Debug 日志面板与全局 Tracing 日志同步。
//!
//! 将内存 RingBuffer 中的实时结构化诊断日志映射为 Slint 数据模型并推送到前端界面。

use std::sync::atomic::{AtomicU64, Ordering};
use slint::ComponentHandle;
use crate::common::to_model_rc;
use crate::generated::{AppWindow, LogEntryData};

static LAST_SYNCED_LOG_VERSION: AtomicU64 = AtomicU64::new(u64::MAX);

/// 同步全局 Tracing 实时事件日志到 Slint UI 调试抽屉。
///
/// 从全局互斥环形缓冲区中提取所有日志条目，将其转化为 Slint 的 `LogEntryData` 向量并刷新 UI。
/// 当日志版本未发生变动时直接返回，避免无谓的 ModelRc 重建与 Slint 脏重绘。
///
/// # 参数
/// - `w`: Slint 主窗口句柄引用
pub(crate) fn sync_ui_debug_logs(w: &AppWindow) {
    if let Ok(buf) = crate::debug::get_global_log_buffer().lock() {
        let current_version = buf.version();
        if current_version == LAST_SYNCED_LOG_VERSION.load(Ordering::Relaxed) {
            return;
        }

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
        let model = to_model_rc(slint_entries);
        w.global::<crate::generated::DebugBridge>().set_logs(model.clone());
        w.global::<crate::generated::WindowBridge>().set_debug_logs(model);
        LAST_SYNCED_LOG_VERSION.store(current_version, Ordering::Relaxed);
    }
}

/// 卸载 Slint UI 绑定的调试日志模型，释放前端条目内存。
pub(crate) fn unload_ui_debug_logs(w: &AppWindow) {
    let empty_model = slint::ModelRc::default();
    w.global::<crate::generated::DebugBridge>().set_logs(empty_model.clone());
    w.global::<crate::generated::WindowBridge>().set_debug_logs(empty_model);
    LAST_SYNCED_LOG_VERSION.store(u64::MAX, Ordering::Relaxed);
    tracing::debug!(target: "smalux::lifecycle", "已释放 Debug 日志 Slint 模型缓存");
}

