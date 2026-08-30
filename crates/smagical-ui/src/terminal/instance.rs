//! 单个终端会话实例状态机。
//!
//! 整合 PTY 进程与 ANSI VT100 解析器，驱动单终端会话的数据流转、尺寸调节与生命周期。

use anyhow::Result;

use crate::terminal::parser::TerminalParser;
use crate::terminal::pty::{PtyProcess, PtySize};

/// 单个活跃终端会话的核心运行时实例。
pub struct TerminalInstance {
    /// 会话唯一标识 ID (如: "sess-1")
    pub session_id: String,
    /// 目标终端或主机的展示名称 (如: "PowerShell 7 #1")
    pub display_name: String,
    /// 底层 PTY 进程句柄
    pub pty: PtyProcess,
    /// ANSI / VT100 字符状态机解析器
    pub parser: TerminalParser,
    /// 当前生效的行列几何尺寸
    pub size: PtySize,
}

impl TerminalInstance {
    /// 启动本地 Shell 终端会话实例。
    ///
    /// # 参数
    /// - `session_id`: 会话唯一标识 ID
    /// - `shell_id`: 本地 Shell 类型标识
    /// - `display_name`: Tab 展示名称
    /// - `cols`: 初始字符列数
    /// - `rows`: 初始字符行数
    pub fn spawn_local(
        session_id: String,
        shell_id: &str,
        display_name: String,
        cols: u16,
        rows: u16,
    ) -> Result<Self> {
        let size = PtySize {
            cols: cols.max(10),
            rows: rows.max(5),
            pixel_width: 0,
            pixel_height: 0,
        };

        let pty = PtyProcess::spawn_local_shell(shell_id, size)?;
        let parser = TerminalParser::new(size.cols, size.rows);

        Ok(Self {
            session_id,
            display_name,
            pty,
            parser,
            size,
        })
    }

    /// 启动远程 SSH 交互式终端会话实例。
    ///
    /// # 参数
    /// - `session_id`: 会话唯一标识 ID
    /// - `display_name`: Tab 展示名称
    /// - `host`: 目标远程主机 IPv4/IPv6 或域名
    /// - `port`: SSH 监听端口
    /// - `username`: 登录用户名 (可选)
    /// - `cols`: 初始字符列数
    /// - `rows`: 初始字符行数
    pub fn spawn_ssh(
        session_id: String,
        display_name: String,
        host: &str,
        port: u16,
        username: Option<&str>,
        cols: u16,
        rows: u16,
    ) -> Result<Self> {
        let size = PtySize {
            cols: cols.max(10),
            rows: rows.max(5),
            pixel_width: 0,
            pixel_height: 0,
        };

        let pty = PtyProcess::spawn_ssh(host, port, username, size)?;
        let parser = TerminalParser::new(size.cols, size.rows);

        Ok(Self {
            session_id,
            display_name,
            pty,
            parser,
            size,
        })
    }


    /// 向终端子进程发送键盘按键字符或转义序列。
    ///
    /// # 参数
    /// - `text`: 键盘输入的 UTF-8 文本或控制字符
    pub fn send_input(&mut self, text: &str) -> Result<()> {
        self.pty.write_str(text)
    }

    /// 向终端子进程发送原始控制字节（如快捷键组合转义码）。
    ///
    /// # 参数
    /// - `bytes`: 待发送的原始字节切片
    pub fn send_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        self.pty.write_bytes(bytes)
    }

    /// 轮询接收 PTY 的输出数据流并喂入 VT100 解析器。
    ///
    /// # 返回值
    /// 若产生新的屏幕变更（需要触发重绘）则返回 `true`；无变更则返回 `false`。
    pub fn poll_output(&mut self) -> bool {
        let chunks = self.pty.try_recv_output();
        if chunks.is_empty() {
            return false;
        }

        for chunk in chunks {
            self.parser.process(&chunk);
        }

        true
    }

    /// 动态伸缩终端视口网格尺寸。
    ///
    /// # 参数
    /// - `cols`: 新的列数 (自适应视口宽度)
    /// - `rows`: 新的行数 (自适应视口高度)
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        let cols = cols.max(10);
        let rows = rows.max(5);

        if self.size.cols == cols && self.size.rows == rows {
            return Ok(());
        }

        let new_size = PtySize {
            cols,
            rows,
            pixel_width: 0,
            pixel_height: 0,
        };

        self.pty.resize(new_size)?;
        self.parser.resize(cols, rows);
        self.size = new_size;

        Ok(())
    }

    /// 向终端发送标准清屏命令序列。
    pub fn clear(&mut self) -> Result<()> {
        self.send_bytes(b"\x1b[2J\x1b[H")?;
        self.parser.clear();
        Ok(())
    }

    /// 查询终端子进程是否存活。
    pub fn is_alive(&mut self) -> bool {
        self.pty.is_alive()
    }

    /// 视口按行增量滚动历史记录 (delta > 0 向上浏览历史, delta < 0 向下返回最新)。
    pub fn scroll_delta(&mut self, delta_lines: i32) {
        self.parser.scroll_delta(delta_lines);
    }

    /// 视口向上翻页 (Page Up)。
    pub fn scroll_page_up(&mut self) {
        self.parser.scroll_page_up();
    }

    /// 视口向下翻页 (Page Down)。
    pub fn scroll_page_down(&mut self) {
        self.parser.scroll_page_down();
    }

    /// 视口滚动至历史最顶端。
    pub fn scroll_to_top(&mut self) {
        self.parser.scroll_to_top();
    }

    /// 视口滚动至最新输出底端。
    pub fn scroll_to_bottom(&mut self) {
        self.parser.scroll_to_bottom();
    }

    /// 获取历史滚动信息 `(history_size, display_offset)`。
    pub fn scroll_info(&self) -> (usize, usize) {
        self.parser.scroll_info()
    }

    /// 提取会话终端屏幕与回滚历史的纯文本快照 (最多保留 max_lines 行，0 为不限)
    pub fn snapshot_text(&self, max_lines: usize) -> String {
        self.parser.extract_all_text(max_lines)
    }
}


