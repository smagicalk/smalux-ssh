//! 基于 alacritty_terminal 的工业级虚拟终端状态机与网格管理。
//!
//! 负责消费来自 PTY/SSH 的 ANSI/DEC/xterm 字节流，维护二维字符网格、回滚历史、光标、样式属性与选区。

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::Processor;

/// 终端几何网格尺寸结构体，用于满足 alacritty_terminal 的 `Dimensions` trait。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TermDimensions {
    /// 终端字符列数
    pub columns: usize,
    /// 终端可见屏幕行数
    pub screen_lines: usize,
}

impl Dimensions for TermDimensions {
    fn total_lines(&self) -> usize {
        self.screen_lines
    }

    fn screen_lines(&self) -> usize {
        self.screen_lines
    }

    fn columns(&self) -> usize {
        self.columns
    }
}

/// 终端事件接收器，用于捕获 alacritty_terminal 回传给 PTY 的控制序列 (如光标位置报告 CPR 等)。
#[derive(Clone, Debug, Default)]
pub struct TerminalEventListener {
    pty_tx: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

impl EventListener for TerminalEventListener {
    fn send_event(&self, event: Event) {
        if let Event::PtyWrite(text) = event {
            tracing::debug!(target: "smagical_ui::terminal", "向 PTY 回写控制序列 (CPR 等): {:?}", text);
            if let Ok(mut guard) = self.pty_tx.lock() {
                if guard.len() < 128 {
                    guard.push(text);
                } else {
                    tracing::warn!(target: "smagical_ui::terminal", "PTY 回写队列达到容量上限 128，丢弃溢出事件");
                }
            }
        }
    }
}

/// 工业级 Alacritty 终端状态机封装。
pub struct TerminalParser {
    /// Alacritty 终端核心状态机
    term: Term<TerminalEventListener>,
    /// VTE ANSI 转义序列处理器
    processor: Processor,
    /// 终端内容脏标记 (是否有新内容需要重绘)
    dirty: bool,
    /// 鼠标划选的屏幕坐标选区 `Some(((start_col, start_row), (end_col, end_row)))`
    selection: Option<((usize, usize), (usize, usize))>,
    /// PTY 响应缓冲区 (如 CPR 光标报告)
    pty_out: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
}

impl TerminalParser {
    /// 创建新的 Alacritty 终端状态机实例。
    ///
    /// # 参数
    /// - `cols`: 初始列数 (Columns)
    /// - `rows`: 初始行数 (Rows)
    pub fn new(cols: u16, rows: u16) -> Self {
        let dimensions = TermDimensions {
            columns: cols as usize,
            screen_lines: rows as usize,
        };

        let pty_out = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let listener = TerminalEventListener {
            pty_tx: std::sync::Arc::clone(&pty_out),
        };

        let config = Config::default();
        let term = Term::new(config, &dimensions, listener);
        let processor = Processor::new();

        Self {
            term,
            processor,
            dirty: true,
            selection: None,
            pty_out,
        }
    }

    /// 提取待向 PTY 写回的协议控制序列 (如 CPR 光标响应)。
    pub fn take_pty_writes(&self) -> Vec<String> {
        if let Ok(mut guard) = self.pty_out.lock() {
            std::mem::take(&mut *guard)
        } else {
            Vec::new()
        }
    }


    /// 消费来自 PTY 的原始 ANSI 字节流并推进状态机。
    ///
    /// # 参数
    /// - `bytes`: 待解析的字节序列切片
    pub fn process(&mut self, bytes: &[u8]) {
        if !bytes.is_empty() {
            self.processor.advance(&mut self.term, bytes);
            self.dirty = true;
        }
    }

    /// 动态调整终端网格行列尺寸。
    ///
    /// # 参数
    /// - `cols`: 新的列数
    /// - `rows`: 新的行数
    pub fn resize(&mut self, cols: u16, rows: u16) {
        let dimensions = TermDimensions {
            columns: cols as usize,
            screen_lines: rows as usize,
        };
        self.term.resize(dimensions);
        self.dirty = true;
    }

    /// 获取当前光标位置 `(col, row)` (0-indexed)。
    pub fn cursor_point(&self) -> (usize, usize) {
        let point = self.term.grid().cursor.point;
        (point.column.0, point.line.0 as usize)
    }

    /// 获取终端网格当前尺寸 `(cols, rows)`。
    pub fn size(&self) -> (usize, usize) {
        (self.term.columns(), self.term.screen_lines())
    }

    /// 检查并重置脏标记。
    pub fn take_dirty(&mut self) -> bool {
        let d = self.dirty;
        self.dirty = false;
        d
    }

    /// 标记需要重绘。
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// 获取当前底层 Term 的不可变引用。
    pub fn term(&self) -> &Term<TerminalEventListener> {
        &self.term
    }

    /// 获取当前底层 Term 的可变引用。
    pub fn term_mut(&mut self) -> &mut Term<TerminalEventListener> {
        &mut self.term
    }

    /// 清空终端视口内容。
    pub fn clear(&mut self) {
        self.process(b"\x1b[2J\x1b[H");
    }

    /// 视口按行增量滚动历史记录 (delta > 0 向上滚动浏览历史, delta < 0 向下滚动返回最新输出)。
    pub fn scroll_delta(&mut self, delta_lines: i32) {
        self.term.scroll_display(alacritty_terminal::grid::Scroll::Delta(delta_lines));
        self.dirty = true;
    }

    /// 视口向上翻页 (Page Up)。
    pub fn scroll_page_up(&mut self) {
        self.term.scroll_display(alacritty_terminal::grid::Scroll::PageUp);
        self.dirty = true;
    }

    /// 视口向下翻页 (Page Down)。
    pub fn scroll_page_down(&mut self) {
        self.term.scroll_display(alacritty_terminal::grid::Scroll::PageDown);
        self.dirty = true;
    }

    /// 视口滚动至历史记录最顶端。
    pub fn scroll_to_top(&mut self) {
        self.term.scroll_display(alacritty_terminal::grid::Scroll::Top);
        self.dirty = true;
    }

    /// 视口滚动至最新输出底端。
    pub fn scroll_to_bottom(&mut self) {
        self.term.scroll_display(alacritty_terminal::grid::Scroll::Bottom);
        self.dirty = true;
    }

    /// 获取历史缓冲区总行数与当前向上滚动的行偏移量 `(history_size, display_offset)`。
    pub fn scroll_info(&self) -> (usize, usize) {
        (self.term.history_size(), self.term.grid().display_offset())
    }

    /// 视口滚动至指定绝对历史偏移量 (target_offset: 0 为最新输出底端, history_size 为最顶端)。
    pub fn scroll_to_offset(&mut self, target_offset: usize) {
        let history = self.term.history_size();
        let target = target_offset.min(history);
        let current = self.term.grid().display_offset();
        let diff = target as i32 - current as i32;
        if diff != 0 {
            self.term.scroll_display(alacritty_terminal::grid::Scroll::Delta(diff));
            self.dirty = true;
        }
    }

    /// 设置屏幕鼠标划选选区 `(start_col, start_row)` 到 `(end_col, end_row)`。
    pub fn set_selection(&mut self, start: (usize, usize), end: (usize, usize)) {
        self.selection = Some((start, end));
        self.dirty = true;
    }

    /// 清除当前鼠标选区。
    pub fn clear_selection(&mut self) {
        if self.selection.is_some() {
            self.selection = None;
            self.dirty = true;
        }
    }

    /// 获取当前选区屏幕坐标。
    pub fn selection(&self) -> Option<((usize, usize), (usize, usize))> {
        self.selection
    }

    /// 提取并返回当前选区覆盖的所有字符拼接而成的纯文本。
    pub fn copy_selection_text(&self) -> String {
        let Some(((c1, r1), (c2, r2))) = self.selection else {
            return String::new();
        };

        // 规范化选区起点与终点，使 (start_row, start_col) <= (end_row, end_col)
        let ((start_row, start_col), (end_row, end_col)) = if r1 < r2 || (r1 == r2 && c1 <= c2) {
            ((r1, c1), (r2, c2))
        } else {
            ((r2, c2), (r1, c1))
        };

        let display_offset = self.term.grid().display_offset() as i32;
        let content = self.term.renderable_content();
        let cols = self.term.columns();
        let rows = self.term.screen_lines();

        let mut line_chars: std::collections::BTreeMap<usize, Vec<(usize, char)>> = std::collections::BTreeMap::new();

        for cell in content.display_iter {
            let col = cell.point.column.0;
            let screen_row_i32 = cell.point.line.0 + display_offset;
            if screen_row_i32 < 0 || screen_row_i32 as usize >= rows || col >= cols {
                continue;
            }
            let row = screen_row_i32 as usize;

            let in_selection = if row < start_row || row > end_row {
                false
            } else if start_row == end_row {
                col >= start_col && col <= end_col
            } else if row == start_row {
                col >= start_col
            } else if row == end_row {
                col <= end_col
            } else {
                true
            };

            if in_selection {
                line_chars.entry(row).or_default().push((col, cell.c));
            }
        }

        let mut lines = Vec::new();
        for (_, mut chars) in line_chars {
            chars.sort_by_key(|(c, _)| *c);
            let mut line_str = String::new();
            for (_, ch) in chars {
                if ch != '\0' {
                    line_str.push(ch);
                } else {
                    line_str.push(' ');
                }
            }
            let trimmed = line_str.trim_end();
            lines.push(trimmed.to_string());
        }

        lines.join("\r\n")
    }

    /// 提取终端当前屏幕以及回滚缓冲区的纯文本快照 (最多保留最新的 max_lines 行，0 为不限)
    pub fn extract_all_text(&self, max_lines: usize) -> String {
        let content = self.term.renderable_content();
        let cols = self.term.columns();

        let mut line_chars: std::collections::BTreeMap<i32, Vec<(usize, char)>> = std::collections::BTreeMap::new();

        for cell in content.display_iter {
            let col = cell.point.column.0;
            let line_i32 = cell.point.line.0;
            if col < cols {
                line_chars.entry(line_i32).or_default().push((col, cell.c));
            }
        }

        let mut lines = Vec::new();
        for (_, mut chars) in line_chars {
            chars.sort_by_key(|(c, _)| *c);
            let mut line_str = String::new();
            for (_, ch) in chars {
                if ch != '\0' {
                    line_str.push(ch);
                } else {
                    line_str.push(' ');
                }
            }
            let trimmed = line_str.trim_end();
            lines.push(trimmed.to_string());
        }

        // 去除尾部多余空行
        while let Some(last) = lines.last() {
            if last.is_empty() {
                lines.pop();
            } else {
                break;
            }
        }

        if max_lines > 0 && lines.len() > max_lines {
            lines[lines.len() - max_lines..].join("\r\n")
        } else {
            lines.join("\r\n")
        }
    }

    /// 提取指定屏幕行号的纯文本字符串 (row: 0..screen_lines)
    pub fn extract_screen_line_text(&self, screen_row: usize) -> String {
        let display_offset = self.term.grid().display_offset() as i32;
        let content = self.term.renderable_content();
        let cols = self.term.columns();
        let rows = self.term.screen_lines();
        if screen_row >= rows {
            return String::new();
        }

        let mut chars = Vec::new();
        for cell in content.display_iter {
            let col = cell.point.column.0;
            let screen_r = cell.point.line.0 + display_offset;
            if screen_r == screen_row as i32 && col < cols {
                chars.push((col, cell.c));
            }
        }
        chars.sort_by_key(|(c, _)| *c);
        let mut s = String::new();
        let mut last_col = 0;
        for (c, ch) in chars {
            while last_col < c {
                s.push(' ');
                last_col += 1;
            }
            s.push(if ch != '\0' { ch } else { ' ' });
            last_col += 1;
        }
        s.trim_end().to_string()
    }

    /// 根据字符网格坐标反查识别所在词、URL 或 IP 地址
    pub fn detect_word_or_url_at(
        &self,
        col: usize,
        row: usize,
        engine: &crate::terminal::highlight::HighlightEngine,
    ) -> Option<(String, bool)> {
        let line_text = self.extract_screen_line_text(row);
        engine.detect_url_or_ip_at(&line_text, col)
    }

    /// 一键将终端当前可视窗口全部字符划选为选区 (Select All)
    pub fn select_all(&mut self) {
        let (cols, rows) = self.size();
        if cols > 0 && rows > 0 {
            self.set_selection((0, 0), (cols.saturating_sub(1), rows.saturating_sub(1)));
        }
    }

    /// 在终端历史回滚缓冲区与当前屏幕网格中检索关键词的所有匹配项
    pub fn find_matches(&self, query: &str, match_case: bool) -> Vec<TerminalSearchMatch> {
        let trimmed_query = query.trim();
        if trimmed_query.is_empty() {
            return Vec::new();
        }

        let query_needle = if match_case {
            trimmed_query.to_string()
        } else {
            trimmed_query.to_lowercase()
        };

        let content = self.term.renderable_content();
        let cols = self.term.columns();
        let mut line_chars: std::collections::BTreeMap<i32, Vec<(usize, char)>> = std::collections::BTreeMap::new();

        for cell in content.display_iter {
            let col = cell.point.column.0;
            let line_i32 = cell.point.line.0;
            if col < cols {
                line_chars.entry(line_i32).or_default().push((col, cell.c));
            }
        }

        let mut matches = Vec::new();
        for (line_i32, mut chars) in line_chars {
            chars.sort_by_key(|(c, _)| *c);
            let mut line_str = String::new();
            let mut col_map = Vec::new();
            for (col, ch) in chars {
                let actual_ch = if ch != '\0' { ch } else { ' ' };
                line_str.push(actual_ch);
                col_map.push(col);
            }

            let search_target = if match_case {
                line_str.clone()
            } else {
                line_str.to_lowercase()
            };

            let mut start_pos = 0;
            while let Some(byte_idx) = search_target[start_pos..].find(&query_needle) {
                let abs_byte_idx = start_pos + byte_idx;
                let char_start = search_target[..abs_byte_idx].chars().count();
                let char_len = query_needle.chars().count();
                let char_end = char_start + char_len;

                let col_start = col_map.get(char_start).copied().unwrap_or(0);
                let col_end = col_map.get(char_end.saturating_sub(1)).copied().unwrap_or(col_start + char_len.saturating_sub(1));

                matches.push(TerminalSearchMatch {
                    line: line_i32,
                    start_col: col_start,
                    end_col: col_end,
                    text: line_str.chars().skip(char_start).take(char_len).collect(),
                });

                start_pos = abs_byte_idx + query_needle.len().max(1);
            }
        }

        matches
    }

    /// 将视口聚焦并高亮划选指定的搜索匹配项
    pub fn focus_match(&mut self, m: &TerminalSearchMatch) {
        let rows = self.term.screen_lines();
        let history_size = self.term.history_size();

        let target_offset = if m.line < 0 {
            let raw_offset = (-m.line as usize).saturating_sub(rows / 2);
            raw_offset.min(history_size)
        } else {
            0
        };

        self.scroll_to_offset(target_offset);

        let display_offset = self.term.grid().display_offset() as i32;
        let screen_row_i32 = m.line + display_offset;
        if screen_row_i32 >= 0 && (screen_row_i32 as usize) < rows {
            let screen_row = screen_row_i32 as usize;
            self.set_selection((m.start_col, screen_row), (m.end_col, screen_row));
        }
    }
}

/// 终端单条搜索匹配项
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalSearchMatch {
    /// 所在网格行号 (负数表示回滚历史行，>= 0 表示当前屏幕行)
    pub line: i32,
    /// 匹配起始列 (0-indexed)
    pub start_col: usize,
    /// 匹配结束列 (0-indexed, inclusive)
    pub end_col: usize,
    /// 匹配命中的纯文本内容
    pub text: String,
}

/// 终端会话搜索状态机 (Search State Machine)
#[derive(Clone, Debug, Default)]
pub struct TerminalSearchState {
    /// 当前搜索查询词
    pub query: String,
    /// 是否区分大小写
    pub match_case: bool,
    /// 匹配项列表
    pub matches: Vec<TerminalSearchMatch>,
    /// 当前聚焦的匹配索引 (0-indexed)
    pub current_index: usize,
}

impl TerminalSearchState {
    /// 创建全新的搜索状态
    pub fn new(query: &str, match_case: bool, matches: Vec<TerminalSearchMatch>) -> Self {
        Self {
            query: query.to_string(),
            match_case,
            matches,
            current_index: 0,
        }
    }

    /// 获取匹配总数
    pub fn total_matches(&self) -> usize {
        self.matches.len()
    }

    /// 获取当前匹配项基于 1 的序号 (若无匹配返回 0)
    pub fn current_1_based(&self) -> usize {
        if self.matches.is_empty() {
            0
        } else {
            self.current_index + 1
        }
    }

    /// 获取当前聚焦的匹配项
    pub fn current_match(&self) -> Option<&TerminalSearchMatch> {
        self.matches.get(self.current_index)
    }

    /// 循环导航到下一个匹配项
    pub fn next_match(&mut self) -> Option<&TerminalSearchMatch> {
        if self.matches.is_empty() {
            return None;
        }
        self.current_index = (self.current_index + 1) % self.matches.len();
        self.matches.get(self.current_index)
    }

    /// 循环导航到上一个匹配项
    pub fn prev_match(&mut self) -> Option<&TerminalSearchMatch> {
        if self.matches.is_empty() {
            return None;
        }
        if self.current_index == 0 {
            self.current_index = self.matches.len() - 1;
        } else {
            self.current_index -= 1;
        }
        self.matches.get(self.current_index)
    }
}




