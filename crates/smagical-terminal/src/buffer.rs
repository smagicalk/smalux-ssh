//! 终端输出缓冲和搜索操作。

use smagical_core::SessionId;

use super::{TerminalManager, TerminalSearchMatch};

impl TerminalManager {
    /// 立即显示用户提交的终端输入，让交互反馈接近真实终端。
    pub fn append_local_echo(&mut self, session_id: SessionId, prompt: &str, input: &str) -> bool {
        let line = format!("{prompt} {}", input.trim_end_matches(['\r', '\n']));
        self.append_output(session_id, line)
    }

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

    /// 清空指定终端的输出缓冲。
    pub fn clear_output(&mut self, session_id: SessionId) -> bool {
        if let Some(tab) = self.tab_mut(session_id) {
            let had_output = !tab.buffer.is_empty();
            tab.buffer.clear();
            had_output
        } else {
            false
        }
    }

    /// 如果 PTY 回传的 echo 与刚刚本地显示的命令重复，则丢弃重复行。
    pub fn suppress_duplicate_echo(
        &mut self,
        session_id: SessionId,
        prompt: &str,
        echoed_line: &str,
    ) -> bool {
        let Some(tab) = self.tab_mut(session_id) else {
            return false;
        };
        let Some(last_line) = tab.buffer.last() else {
            return false;
        };
        let Some(command) = last_line.strip_prefix(prompt).map(str::trim_start) else {
            return false;
        };

        if command == echoed_line.trim() {
            return true;
        }

        false
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
    use crate::TerminalTabState;
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
    fn clear_output_removes_terminal_buffer() {
        let mut terminal = TerminalManager::default();
        let id = session_id();

        terminal.open_tab(TerminalTabState::new(id, "production"));
        terminal.append_output(id, "first");
        terminal.append_output(id, "second");

        assert!(terminal.clear_output(id));
        assert!(terminal.tabs[0].buffer.is_empty());
        assert!(!terminal.clear_output(id));
        assert!(!terminal.clear_output(session_id()));
    }

    #[test]
    fn local_echo_is_visible_immediately_and_can_drop_duplicate_shell_echo() {
        let mut terminal = TerminalManager::default();
        let id = session_id();

        terminal.open_tab(TerminalTabState::new(id, "local"));

        assert!(terminal.append_local_echo(id, "PS>", "ls\n"));
        assert_eq!(terminal.tabs[0].buffer, vec!["PS> ls"]);
        assert!(terminal.suppress_duplicate_echo(id, "PS>", "ls"));
        assert!(!terminal.suppress_duplicate_echo(id, "PS>", "dir"));
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
