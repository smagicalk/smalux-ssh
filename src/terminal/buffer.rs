//! 终端输出缓冲和搜索操作。

use crate::model::SessionId;

use super::{TerminalManager, TerminalSearchMatch};

impl TerminalManager {
    /// 追加终端输出行，并按滚动缓冲上限裁剪。
    pub fn append_output(&mut self, session_id: SessionId, line: impl Into<String>) -> bool {
        if let Some(tab) = self.tab_mut(session_id) {
            tab.buffer.push(line.into());
            tab.trim_scrollback();
            true
        } else {
            false
        }
    }

    /// 搜索终端缓冲区。
    pub fn search(&self, session_id: SessionId, query: &str) -> Vec<TerminalSearchMatch> {
        let query = query.trim();

        if query.is_empty() {
            return Vec::new();
        }

        self.tabs
            .iter()
            .find(|tab| tab.session_id == session_id)
            .map(|tab| tab.search(query))
            .unwrap_or_default()
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
    fn append_output_respects_scrollback_limit() {
        let mut terminal = TerminalManager::default();
        let id = session_id();
        let mut tab = TerminalTabState::new(id, "production");
        tab.scrollback_limit = 2;

        terminal.open_tab(tab);

        assert!(terminal.append_output(id, "first"));
        assert!(terminal.append_output(id, "second"));
        assert!(terminal.append_output(id, "third"));

        assert_eq!(terminal.tabs[0].buffer, vec!["second", "third"]);
    }

    #[test]
    fn search_returns_line_and_column_matches() {
        let mut terminal = TerminalManager::default();
        let id = session_id();

        terminal.open_tab(TerminalTabState::new(id, "production"));
        terminal.append_output(id, "starting sshd");
        terminal.append_output(id, "sshd is running");

        let matches = terminal.search(id, "sshd");

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line_index, 0);
        assert_eq!(matches[0].column, 9);
        assert!(terminal.search(id, "").is_empty());
    }
}
