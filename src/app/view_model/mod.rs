//! 核心状态到 UI 展示模型的转换。
//!
//! 这里是核心领域层和 Slint 表现层之间的边界，避免 Slint 投影直接散落业务字段读取。

mod activity;
mod common;
mod hosts;
mod labels;
mod palette;
mod root;
mod sftp;
mod tabs;
#[cfg(test)]
mod tests;
mod tools;

pub(super) use activity::ActivityViewModel;
pub(super) use hosts::HostViewModel;
pub(super) use palette::CommandPaletteItemViewModel;
pub(super) use root::{AppViewModel, app_view_model};
pub(super) use sftp::SftpEntryViewModel;
pub(super) use tabs::active_terminal;
pub(super) use tabs::{SessionTabViewModel, TerminalViewModel};
pub(super) use tools::{KnownHostViewModel, ToolItemViewModel};
