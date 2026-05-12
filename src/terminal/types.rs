//! 终端状态的基础数据类型。

use crate::model::SessionId;

pub(crate) const DEFAULT_COLUMNS: u16 = 120;
pub(crate) const DEFAULT_ROWS: u16 = 32;
pub(crate) const DEFAULT_SCROLLBACK_LIMIT: usize = 10_000;

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

    pub(crate) fn trim_scrollback(&mut self) {
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

    #[test]
    fn terminal_size_rejects_zero_dimensions() {
        let size = TerminalSize::new(0, 0);

        assert_eq!(size.columns, 1);
        assert_eq!(size.rows, 1);
    }

    #[test]
    fn terminal_tab_starts_with_default_runtime_state() {
        let id = SessionId(Uuid::new_v4());
        let tab = TerminalTabState::new(id, "production");

        assert_eq!(tab.session_id, id);
        assert_eq!(tab.title, "production");
        assert_eq!(tab.size, TerminalSize::default());
        assert!(tab.buffer.is_empty());
        assert_eq!(tab.scrollback_limit, DEFAULT_SCROLLBACK_LIMIT);
        assert!(tab.selection.is_none());
    }
}
