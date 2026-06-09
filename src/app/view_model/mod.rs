//! 核心状态到 UI 展示模型的转换。
//!
//! 这里是核心层和表现层之间的展示边界。它读取 `AppState`，输出不含 Slint
//! 类型的普通 Rust 展示模型。
//!
//! 设计目标：
//!
//! - UI 可以重写。新的 UI 可以复用这些 view model，也可以只复用其中一部分。
//! - 展示文本集中处理。i18n、状态 key、主题名、列表排序和过滤都不要散落在
//!   Slint 文件里。
//! - 逻辑 key 与展示 label 分离。例如 `kind_key = "Host"` 用于点击分发，
//!   `kind = "主机"` 用于显示，避免中文模式破坏操作逻辑。
//! - 核心状态保持可测试。这里的转换函数是 UI 层测试的主要入口。

mod activity;
mod common;
mod hosts;
mod i18n;
mod labels;
mod palette;
mod root;
mod settings;
mod sftp;
mod tabs;
#[cfg(test)]
mod tests;
mod tools;
mod tools_credentials;
mod tools_credentials_common;
mod tools_credentials_detail;
mod tools_credentials_group_content;
mod tools_credentials_tree;
mod tools_known_hosts;
mod tools_snippets;
mod tools_tunnels;
mod tools_types;

pub(super) use activity::ActivityViewModel;
pub(super) use hosts::{
    CredentialOptionViewModel, GroupOptionViewModel, HostTreeViewModel, HostViewModel,
};
pub(super) use palette::CommandPaletteItemViewModel;
pub(super) use root::{AppViewModel, app_view_model};
pub(super) use settings::{
    CustomThemeProfileViewModel, SettingOptionViewModel, SettingsFileActionViewModel,
    SettingsStorageSummaryItemViewModel,
};
pub(super) use sftp::SftpEntryViewModel;
pub(super) use tabs::active_terminal;
pub(super) use tabs::{SessionTabViewModel, TerminalViewModel};
pub(super) use tools_types::{
    CredentialDetailFieldViewModel, CredentialGroupContentViewModel, CredentialRowViewModel,
    KnownHostViewModel, SnippetRowViewModel, ToolItemViewModel,
};
