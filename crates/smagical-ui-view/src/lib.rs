//! smagical-ui-view
//!
//! 轻量门面仓 (Facade)，聚合微内核底座与全部业务插件，保持上层调用兼容性。

#![allow(missing_docs, dead_code)]

pub use smagical_ui_kernel::*;

/// 系统托盘图标 PNG 静态字节数据
pub static TRAY_PNG_BYTES: &[u8] = include_bytes!("../../ui/common/ui/assets/tray-icon.png");

/// 终端渲染默认字体 JetBrains Mono 静态字节数据
pub static JETBRAINS_MONO_BYTES: &[u8] = include_bytes!("../../ui/common/ui/assets/fonts/JetBrainsMono-Regular.ttf");
