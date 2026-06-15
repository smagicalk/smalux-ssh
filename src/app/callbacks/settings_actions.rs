//! 设置页操作回调。
//!
//! 设置页回调把 UI 的选择、导入导出路径和格式 key 转换为核心消息。这里不直接读写主题
//! 文件或 SQLite 文件，真正的文件操作在 `CoreState`/storage/theme 模块中完成。

use super::{AppWindow, SharedAppState};

#[path = "settings_storage_actions.rs"]
mod settings_storage_actions;
#[path = "settings_theme_actions.rs"]
mod settings_theme_actions;

pub(super) fn bind(window: &AppWindow, state: SharedAppState) {
    // 文件选择属于 Slint adapter：核心层只接收最终路径，不依赖桌面 UI 库。
    window.on_choose_file_action_path(|_key, current_path, direction, extension| {
        crate::app::file_dialog::choose_settings_file_action_path(
            current_path.as_str(),
            direction.as_str(),
            extension.as_str(),
        )
        .into()
    });

    settings_theme_actions::bind(window, &state);
    settings_storage_actions::bind(window, &state);
}
