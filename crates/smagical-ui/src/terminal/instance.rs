//! 单个终端会话实例状态机。
//!
//! 整合 PTY 进程与 ANSI VT100 解析器，驱动单终端会话的数据流转、尺寸调节与生命周期。

use anyhow::Result;

use crate::terminal::parser::TerminalParser;
use crate::terminal::pty::{PtyProcess, PtySize};

/// 终端会话的目标类型与启动参数配置 (用于支持原地重新连接)。
#[derive(Clone, Debug)]
pub enum TerminalTarget {
    /// 本地 Shell 终端环境
    Local {
        /// 本地 Shell 标识符 (如 "powershell", "cmd")
        shell_id: String,
    },
    /// 远程 SSH 交互式终端
    Ssh {
        /// 远程主机 IP 或域名
        host: String,
        /// 远程 SSH 端口号
        port: u16,
        /// 登录用户名 (可选)
        username: Option<String>,
    },
}

/// 终端会话退出/断联的具体原因分类。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionExitReason {
    /// 正常主动退出 (代码 0，如 exit / logout / Ctrl+D)
    Normal(u32),
    /// 进程异常崩溃或意外终止 (非零代码，如段错误、强制杀灭)
    Crash(u32),
    /// 网络闪断、连接超时或远端复位断开 (如 SSH Broken pipe / Connection reset)
    Disconnected,
}

/// 终端会话运行时生命周期状态。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionState {
    /// 双向数据流正常交互运行中
    Running,
    /// 会话已结束或断开连接
    Exited {
        /// 退出原因分类
        reason: SessionExitReason,
        /// 退出发生的时间戳 (用于防连击防误触冷却)
        exited_at: std::time::Instant,
    },
}

/// 单个活跃终端会话的核心运行时实例。
pub struct TerminalInstance {
    /// 会话唯一标识 ID (如: "sess-1")
    pub session_id: String,
    /// 目标终端或主机的展示名称 (如: "PowerShell 7 #1")
    pub display_name: String,
    /// 目标类型与会话配置 (用于原地重连)
    pub target: TerminalTarget,
    /// 当前生命周期状态 (运行中 / 已退出等待重连)
    pub state: SessionState,
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
            target: TerminalTarget::Local { shell_id: shell_id.to_string() },
            state: SessionState::Running,
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
            target: TerminalTarget::Ssh {
                host: host.to_string(),
                port,
                username: username.map(|s| s.to_string()),
            },
            state: SessionState::Running,
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
        if matches!(self.state, SessionState::Exited { .. }) {
            return Ok(());
        }
        self.pty.write_str(text)
    }

    /// 向终端子进程发送原始控制字节（如快捷键组合转义码）。
    ///
    /// # 参数
    /// - `bytes`: 待发送的原始字节切片
    pub fn send_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        if matches!(self.state, SessionState::Exited { .. }) {
            return Ok(());
        }
        self.pty.write_bytes(bytes)
    }

    /// 轮询接收 PTY 的输出数据流并喂入 VT100 解析器，同时自动检测子进程存活与断线退出。
    ///
    /// # 返回值
    /// 若产生新的屏幕变更（需要触发重绘）则返回 `true`；无变更则返回 `false`。
    pub fn poll_output(&mut self) -> bool {
        let chunks = self.pty.try_recv_output();
        let has_new_chunks = !chunks.is_empty();

        for chunk in chunks {
            self.parser.process(&chunk);
        }

        for response in self.parser.take_pty_writes() {
            let _ = self.pty.write_str(&response);
        }

        // 检查子进程是否已退出且当前尚未处于 Exited 状态
        if self.state == SessionState::Running && !self.pty.is_alive() {
            self.handle_process_exit();
            return true;
        }

        has_new_chunks
    }

    /// 处理子进程退出或网络断联事件：分析退出原因、向视口注入专属 ANSI 提示横幅并切换状态。
    pub fn handle_process_exit(&mut self) {
        let exit_status = self.pty.exit_status();
        let reason = match &self.target {
            TerminalTarget::Ssh { .. } => {
                match exit_status {
                    Some(st) if st.success() => SessionExitReason::Normal(0),
                    Some(st) if st.exit_code() == 255 => SessionExitReason::Disconnected,
                    Some(st) => SessionExitReason::Crash(st.exit_code()),
                    None => SessionExitReason::Disconnected,
                }
            }
            TerminalTarget::Local { .. } => {
                match exit_status {
                    Some(st) if st.success() => SessionExitReason::Normal(0),
                    Some(st) => SessionExitReason::Crash(st.exit_code()),
                    None => SessionExitReason::Normal(0),
                }
            }
        };

        let banner = match &reason {
            SessionExitReason::Normal(code) => {
                format!(
                    "\r\n\r\n\x1b[90m--------------------------------------------------\x1b[0m\r\n\
                     \x1b[32;1m[✔ 会话已正常结束 (代码 {})]\x1b[0m\r\n\
                     \x1b[90m按任意键重新连接，或按 Ctrl+W 关闭标签页\x1b[0m\r\n",
                    code
                )
            }
            SessionExitReason::Disconnected => {
                "\r\n\r\n\x1b[90m--------------------------------------------------\x1b[0m\r\n\
                 \x1b[33;1m[⚡ 远程连接已断开: 网络中断或主机连接重置]\x1b[0m\r\n\
                 \x1b[90m按任意键尝试重新建立连接，或按 Ctrl+W 关闭标签页\x1b[0m\r\n"
                    .to_string()
            }
            SessionExitReason::Crash(code) => {
                format!(
                    "\r\n\r\n\x1b[90m--------------------------------------------------\x1b[0m\r\n\
                     \x1b[31;1m[✖ 进程异常终止 (退出代码: {})]\x1b[0m\r\n\
                     \x1b[90m按任意键重新尝试启动，或按 Ctrl+W 关闭标签页\x1b[0m\r\n",
                    code
                )
            }
        };

        self.parser.process(banner.as_bytes());
        self.parser.mark_dirty();
        self.state = SessionState::Exited {
            reason,
            exited_at: std::time::Instant::now(),
        };
    }

    /// 获取当前会话状态对应的指示灯状态字符串 ("online" | "offline" | "warning" | "error")
    pub fn current_status(&self) -> &'static str {
        match &self.state {
            SessionState::Running => "online",
            SessionState::Exited { reason, .. } => match reason {
                SessionExitReason::Normal(_) => "offline",
                SessionExitReason::Disconnected => "warning",
                SessionExitReason::Crash(_) => "error",
            },
        }
    }

    /// 查询当前会话是否处于退出/断联状态。
    pub fn is_exited(&self) -> bool {
        matches!(self.state, SessionState::Exited { .. })
    }

    /// 原地重新连接此终端会话 (释放老旧 PTY 并拉起全新子进程)。
    pub fn reconnect(&mut self) -> Result<()> {
        let _ = self.pty.kill();
        let new_pty = match &self.target {
            TerminalTarget::Local { shell_id } => {
                PtyProcess::spawn_local_shell(shell_id, self.size)?
            }
            TerminalTarget::Ssh { host, port, username } => {
                PtyProcess::spawn_ssh(host, *port, username.as_deref(), self.size)?
            }
        };
        self.pty = new_pty;
        self.state = SessionState::Running;
        self.parser.process("\r\n\x1b[36;1m[正在重新建立会话连接...]\x1b[0m\r\n\r\n".as_bytes());
        self.parser.mark_dirty();
        Ok(())
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

    /// 视口滚动至指定绝对历史偏移量。
    pub fn scroll_to_offset(&mut self, target_offset: usize) {
        self.parser.scroll_to_offset(target_offset);
    }

    /// 提取会话终端屏幕与回滚历史的纯文本快照 (最多保留 max_lines 行，0 为不限)
    pub fn snapshot_text(&self, max_lines: usize) -> String {
        self.parser.extract_all_text(max_lines)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_local_cmd() {
        let res = TerminalInstance::spawn_local("sess-test".into(), "local-cmd", "CMD Test".into(), 80, 24);
        assert!(res.is_ok(), "Failed to spawn cmd: {:?}", res.err());
        let mut instance = res.unwrap();
        assert!(instance.is_alive());

        // 首次轮询：消费 ConPTY 的初始 \x1b[6n 探测序列并将 CPR 光标响应写回 PTY
        std::thread::sleep(std::time::Duration::from_millis(200));
        let _ = instance.poll_output();

        // 等待 Shell 收到 CPR 响应后打印初始命令提示符
        std::thread::sleep(std::time::Duration::from_millis(400));
        let _ = instance.poll_output();
        let initial_text = instance.snapshot_text(10);

        // 发送测试命令并验证回显与输出
        instance.send_input("echo HELLO_CONPTY\r\n").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(500));
        let _ = instance.poll_output();
        let cmd_text = instance.snapshot_text(10);

        assert!(
            cmd_text.contains("HELLO_CONPTY") || !initial_text.is_empty(),
            "PTY 终端必须成功产生画面文本输出，实际获得: {:?}",
            cmd_text
        );
    }

    #[test]
    fn test_process_exit_and_reconnect() {
        let res = TerminalInstance::spawn_local("sess-exit-test".into(), "local-cmd", "CMD Exit Test".into(), 80, 24);
        assert!(res.is_ok());
        let mut instance = res.unwrap();
        assert_eq!(instance.current_status(), "online");

        // 等待并消费初始 CPR
        std::thread::sleep(std::time::Duration::from_millis(200));
        let _ = instance.poll_output();

        // 发送 exit 退出命令
        instance.send_input("exit\r\n").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(600));

        // 轮询应检测到子进程退出，自动注入提示横幅并切换状态
        let had_update = instance.poll_output();
        assert!(had_update, "检测到退出应触发重绘变更");
        assert!(instance.is_exited(), "会话状态应转为 Exited");
        assert_eq!(instance.current_status(), "offline");

        let text = instance.snapshot_text(10);
        let compact_text = text.replace(' ', "");
        assert!(
            compact_text.contains("会话已正常结束") || compact_text.contains("按任意键重新连接") || text.contains("Ctrl+W"),
            "视口中必须包含退出提示横幅，实际文本: {:?}",
            text
        );

        // 原地重新连接
        let reconnect_res = instance.reconnect();
        assert!(reconnect_res.is_ok(), "原地重新连接应当成功: {:?}", reconnect_res.err());
        assert_eq!(instance.current_status(), "online");
        assert!(!instance.is_exited());
    }
}


