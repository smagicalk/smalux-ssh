//! 设置页存储和备份回调。
//!
//! 这里把设置页选择的备份、快照和 SQLite 导入路径转成核心 `Message`。实际文件读写
//! 留在核心状态和 storage 层，避免 UI Adapter 依赖存储实现细节。

use std::rc::Rc;

use slint::ComponentHandle;

use crate::app::callbacks::{AppWindow, SharedAppState, apply_and_sync};
use crate::model::Message;

pub(super) fn bind(window: &AppWindow, state: &SharedAppState) {
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_backup_storage(move |target_path| {
            apply_and_sync(
                &weak,
                &state,
                Message::BackupStorage {
                    target_path: target_path.to_string(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_export_storage_snapshot(move |target_path| {
            apply_and_sync(
                &weak,
                &state,
                Message::ExportStorageSnapshot {
                    target_path: target_path.to_string(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_import_storage_snapshot(move |source_path| {
            apply_and_sync(
                &weak,
                &state,
                Message::ImportStorageSnapshot {
                    source_path: source_path.to_string(),
                },
            );
        });
    }
    {
        let weak = window.as_weak();
        let state = Rc::clone(state);
        window.on_import_sqlite_backup(move |source_path| {
            apply_and_sync(
                &weak,
                &state,
                Message::ImportSqliteBackup {
                    source_path: source_path.to_string(),
                },
            );
        });
    }
}
