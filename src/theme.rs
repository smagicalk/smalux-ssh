//! 可导入导出的 UI 主题配置。

mod builtin;
mod color;
mod document;
mod exchange;
mod partial;
mod resolve;

pub use builtin::{built_in_palette, built_in_theme_document};
pub use document::*;

#[cfg(test)]
mod tests;
