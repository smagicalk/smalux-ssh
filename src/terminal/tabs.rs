//! 终端标签页打开、关闭和尺寸调整。

use crate::model::SessionId;

use super::{TerminalManager, TerminalSize, TerminalTabState};

impl TerminalManager {
    /// 打开或替换一个终端标签页。
    pub fn open_tab(&mut self, tab: TerminalTabState) {
        self.tabs
            .retain(|existing| existing.session_id != tab.session_id);
        self.active_tab = Some(tab.session_id);
        self.tabs.push(tab);
        self.tab_count = self.tabs.len();
    }

    /// 关闭终端标签页。
    pub fn close_tab(&mut self, session_id: SessionId) -> bool {
        let before = self.tabs.len();
        self.tabs.retain(|tab| tab.session_id != session_id);

        if self.active_tab == Some(session_id) {
            self.active_tab = self.tabs.last().map(|tab| tab.session_id);
        }

        self.tab_count = self.tabs.len();
        before != self.tabs.len()
    }

    /// 调整终端尺寸。
    pub fn resize_tab(&mut self, session_id: SessionId, columns: u16, rows: u16) -> bool {
        if let Some(tab) = self.tab_mut(session_id) {
            tab.size = TerminalSize::new(columns, rows);
            true
        } else {
            false
        }
    }

    /// 切换当前活动终端标签页。
    pub fn set_active_tab(&mut self, session_id: SessionId) -> bool {
        if self.tabs.iter().any(|tab| tab.session_id == session_id) {
            self.active_tab = Some(session_id);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn session_id() -> SessionId {
        SessionId(Uuid::new_v4())
    }

    #[test]
    fn opening_tab_updates_count_and_active_tab() {
        let mut terminal = TerminalManager::default();
        let id = session_id();

        terminal.open_tab(TerminalTabState::new(id, "production"));

        assert_eq!(terminal.tab_count(), 1);
        assert_eq!(terminal.tab_count, 1);
        assert_eq!(terminal.active_tab, Some(id));
        assert_eq!(terminal.tabs[0].title, "production");
    }

    #[test]
    fn opening_same_tab_replaces_existing_state() {
        let mut terminal = TerminalManager::default();
        let id = session_id();

        terminal.open_tab(TerminalTabState::new(id, "old"));
        terminal.open_tab(TerminalTabState::new(id, "new"));

        assert_eq!(terminal.tab_count(), 1);
        assert_eq!(terminal.tabs[0].title, "new");
    }

    #[test]
    fn close_tab_updates_active_tab() {
        let mut terminal = TerminalManager::default();
        let first_id = session_id();
        let second_id = session_id();

        terminal.open_tab(TerminalTabState::new(first_id, "first"));
        terminal.open_tab(TerminalTabState::new(second_id, "second"));

        assert!(terminal.close_tab(second_id));
        assert_eq!(terminal.active_tab, Some(first_id));
        assert_eq!(terminal.tab_count(), 1);
        assert!(!terminal.close_tab(second_id));
    }

    #[test]
    fn resize_tab_updates_terminal_size() {
        let mut terminal = TerminalManager::default();
        let id = session_id();

        terminal.open_tab(TerminalTabState::new(id, "production"));

        assert!(terminal.resize_tab(id, 160, 48));
        assert_eq!(terminal.tabs[0].size, TerminalSize::new(160, 48));
        assert!(!terminal.resize_tab(session_id(), 80, 24));
    }

    #[test]
    fn set_active_tab_only_accepts_existing_tabs() {
        let mut terminal = TerminalManager::default();
        let first = session_id();
        let second = session_id();

        terminal.open_tab(TerminalTabState::new(first, "first"));

        assert!(terminal.set_active_tab(first));
        assert_eq!(terminal.active_tab, Some(first));
        assert!(!terminal.set_active_tab(second));
    }
}
