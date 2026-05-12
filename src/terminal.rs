//! 终端标签页状态。
//!
//! alacritty_terminal 和 portable-pty 的具体适配会放在后续终端后端模块；
//! 这个管理器只暴露 UI 需要展示、切换、搜索和恢复的终端状态。

use crate::model::SessionId;

const DEFAULT_COLUMNS: u16 = 120;
const DEFAULT_ROWS: u16 = 32;
const DEFAULT_SCROLLBACK_LIMIT: usize = 10_000;

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

    /// 保存或更新一个本地 shell 配置。
    pub fn upsert_local_shell(&mut self, profile: LocalShellProfile) {
        if let Some(existing) = self
            .local_shells
            .iter_mut()
            .find(|existing| existing.name == profile.name)
        {
            *existing = profile;
        } else {
            self.local_shells.push(profile);
        }
    }

    fn tab_mut(&mut self, session_id: SessionId) -> Option<&mut TerminalTabState> {
        self.tabs
            .iter_mut()
            .find(|tab| tab.session_id == session_id)
    }
}

/// 终端标签页状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalTabState {
    pub session_id: SessionId,
    pub title: String,
    pub size: TerminalSize,
    pub buffer: Vec<String>,
    pub scrollback_limit: usize,
    pub selection: Option<String>,
}

impl TerminalTabState {
    /// 创建一个使用默认尺寸和滚动缓冲的终端标签页。
    pub fn new(session_id: SessionId, title: impl Into<String>) -> Self {
        Self {
            session_id,
            title: title.into(),
            size: TerminalSize::default(),
            buffer: Vec::new(),
            scrollback_limit: DEFAULT_SCROLLBACK_LIMIT,
            selection: None,
        }
    }

    /// 在当前缓冲区中查找文本。
    pub fn search(&self, query: &str) -> Vec<TerminalSearchMatch> {
        self.buffer
            .iter()
            .enumerate()
            .filter_map(|(line_index, line)| {
                line.find(query).map(|column| TerminalSearchMatch {
                    line_index,
                    column,
                    text: query.to_owned(),
                })
            })
            .collect()
    }

    fn trim_scrollback(&mut self) {
        if self.buffer.len() > self.scrollback_limit {
            let overflow = self.buffer.len() - self.scrollback_limit;
            self.buffer.drain(0..overflow);
        }
    }
}

/// 终端字符网格尺寸。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub columns: u16,
    pub rows: u16,
}

impl TerminalSize {
    /// 创建终端尺寸，避免传入 0 导致 PTY 尺寸非法。
    pub fn new(columns: u16, rows: u16) -> Self {
        Self {
            columns: columns.max(1),
            rows: rows.max(1),
        }
    }
}

impl Default for TerminalSize {
    fn default() -> Self {
        Self::new(DEFAULT_COLUMNS, DEFAULT_ROWS)
    }
}

/// 终端搜索结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalSearchMatch {
    pub line_index: usize,
    pub column: usize,
    pub text: String,
}

/// 本地终端入口配置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalShellProfile {
    pub name: String,
    pub program: String,
    pub args: Vec<String>,
    pub working_directory: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn session_id() -> SessionId {
        SessionId(Uuid::new_v4())
    }

    #[test]
    fn default_terminal_manager_has_no_tabs() {
        let terminal = TerminalManager::default();

        assert_eq!(terminal.tab_count(), 0);
        assert_eq!(terminal.local_shell_count(), 0);
        assert!(terminal.active_tab.is_none());
    }

    #[test]
    fn terminal_size_rejects_zero_dimensions() {
        let size = TerminalSize::new(0, 0);

        assert_eq!(size.columns, 1);
        assert_eq!(size.rows, 1);
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

    #[test]
    fn local_shell_profiles_can_be_upserted() {
        let mut terminal = TerminalManager::default();

        terminal.upsert_local_shell(LocalShellProfile {
            name: "PowerShell".to_owned(),
            program: "powershell.exe".to_owned(),
            args: vec!["-NoLogo".to_owned()],
            working_directory: None,
        });
        terminal.upsert_local_shell(LocalShellProfile {
            name: "PowerShell".to_owned(),
            program: "pwsh.exe".to_owned(),
            args: vec!["-NoLogo".to_owned()],
            working_directory: Some("C:/Users".to_owned()),
        });

        assert_eq!(terminal.local_shell_count(), 1);
        assert_eq!(terminal.local_shells[0].program, "pwsh.exe");
        assert_eq!(
            terminal.local_shells[0].working_directory.as_deref(),
            Some("C:/Users")
        );
    }
}
