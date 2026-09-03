//! Slint 主题资源注册与运行时应用适配。

mod apply;
mod builtins;

pub use apply::{apply_theme_by_id, apply_ui_theme, restore_system_theme, sync_ui_themes};
pub use builtins::{builtin_themes, initialize_theme_service};
