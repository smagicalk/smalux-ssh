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

/// 空事件接收器，用于满足 alacritty_terminal 的 EventListener trait 约束。
#[derive(Clone, Copy, Debug, Default)]
pub struct TerminalEventListener;

impl EventListener for TerminalEventListener {
    fn send_event(&self, _event: Event) {}
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

        let config = Config::default();
        let term = Term::new(config, &dimensions, TerminalEventListener);
        let processor = Processor::new();

        Self {
            term,
            processor,
            dirty: true,
            selection: None,
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
}



