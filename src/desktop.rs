//! 当前桌面 UI 门面。
//!
//! 这个模块代表“当前 Slint 桌面适配层”。未来如果切到别的原生 GUI，新的 UI
//! 入口可以替换这里，而 `core` 不需要知道窗口、回调和投影细节。

#![cfg(feature = "desktop")]

pub use crate::app::run;
