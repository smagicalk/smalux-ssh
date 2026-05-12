//! 终端选择文本和复制状态。

use crate::model::SessionId;

use super::TerminalManager;

impl TerminalManager {
    /// 设置当前选择文本。
    pub fn set_selection(&mut self, session_id: SessionId, selection: Option<String>) -> bool {
        if let Some(tab) = self.tab_mut(session_id) {
            tab.selection = selection;
            true
        } else {
            false
        }
    }

    /// 查询当前选择文本。
    pub fn copy_selection(&self, session_id: SessionId) -> Option<&str> {
        self.tabs
            .iter()
            .find(|tab| tab.session_id == session_id)
            .and_then(|tab| tab.selection.as_deref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::TerminalTabState;
    use uuid::Uuid;

    fn session_id() -> SessionId {
        SessionId(Uuid::new_v4())
    }

    #[test]
    fn selection_can_be_set_and_copied() {
        let mut terminal = TerminalManager::default();
        let id = session_id();

        terminal.open_tab(TerminalTabState::new(id, "production"));

        assert!(terminal.set_selection(id, Some("selected text".to_owned())));
        assert_eq!(terminal.copy_selection(id), Some("selected text"));
        assert!(terminal.set_selection(id, None));
        assert_eq!(terminal.copy_selection(id), None);
    }
}
