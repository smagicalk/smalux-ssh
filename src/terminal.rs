//! 终端标签页状态。
//!
//! alacritty_terminal 和 portable-pty 的具体适配会放在后续终端后端模块；
//! 这个管理器只暴露 UI 需要展示、切换、搜索和恢复的终端状态。

mod buffer;
mod selection;
mod shells;
mod tabs;
mod types;

pub use types::*;

use crate::model::SessionId;

/// 终端标签页管理器。
#[derive(Debug, Clone, Default)]
pub struct TerminalManager {
    pub tab_count: usize,
    pub tabs: Vec<TerminalTabState>,
    pub active_tab: Option<SessionId>,
    pub local_shells: Vec<LocalShellProfile>,
}

impl TerminalManager {
    /// 终端标签页数量。
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }

    /// 本地终端入口数量。
    pub fn local_shell_count(&self) -> usize {
        self.local_shells.len()
    }

    pub(crate) fn tab_mut(&mut self, session_id: SessionId) -> Option<&mut TerminalTabState> {
        self.tabs
            .iter_mut()
            .find(|tab| tab.session_id == session_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_terminal_manager_has_no_tabs() {
        let terminal = TerminalManager::default();

        assert_eq!(terminal.tab_count(), 0);
        assert_eq!(terminal.local_shell_count(), 0);
        assert!(terminal.active_tab.is_none());
    }
}
