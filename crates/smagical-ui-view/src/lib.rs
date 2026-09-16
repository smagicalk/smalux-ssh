//! smagical-ui-view
//!
//! 纯 Slint 声明式视图与强类型生成代码库。
//! 集中托管所有 `.slint` 界面声明、静态资源与由 `slint-build` 宏展开生成的全部强类型代码。

#![allow(missing_docs, dead_code)]

slint::include_modules!();

/// 系统托盘图标 PNG 静态字节数据
pub static TRAY_PNG_BYTES: &[u8] = include_bytes!("../ui/assets/tray-icon.png");

/// 终端渲染默认字体 JetBrains Mono 静态字节数据
pub static JETBRAINS_MONO_BYTES: &[u8] = include_bytes!("../ui/assets/fonts/JetBrainsMono-Regular.ttf");
