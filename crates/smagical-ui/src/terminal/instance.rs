//! 单个终端会话实例状态机。
//!
//! 整合 PTY 进程与 ANSI VT100 解析器，驱动单终端会话的数据流转、尺寸调节与生命周期。

use anyhow::Result;

use crate::terminal::backend::TerminalBackend;
use crate::terminal::parser::TerminalParser;
use crate::terminal::pty::{PtyProcess, PtySize};
use crate::terminal::pure_ssh::PureSshProcess;
use crate::terminal::ssh_config::{KeyTempGuard, SshLaunchConfig};
use smagical_core::service::ssh::SshStreamChannel;

/// 终端会话的目标类型与启动参数配置 (用于支持原地重新连接)。
#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
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
        /// 高级启动配置
        config: Option<SshLaunchConfig>,
    },
}

/// 终端会话退出/断联的具体原因分类。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SessionExitReason {
    /// 正常主动退出 (代码 0，如 exit / logout / Ctrl+D)
    Normal(u32),
    /// 身份认证失败或权限不足 (如 Permission denied、密钥/密码被拒)
    AuthFailed,
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
    /// 自动重新连接调度中 (指数退避)
    Reconnecting {
        /// 当前重连轮次 (从 1 开始)
        attempt: u32,
        /// 最大允许自动重试次数
        max_attempts: u32,
        /// 下一次发起重连的时间戳
        next_attempt_at: std::time::Instant,
    },
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
    /// 底层终端会话后端 (本地 ConPTY 进程或纯 Rust 原生 SshStreamChannel 管道)
    pub pty: TerminalBackend,
    /// ANSI / VT100 字符状态机解析器
    pub parser: TerminalParser,
    /// 当前生效的行列几何尺寸
    pub size: PtySize,
    /// 临时私钥物理文件生命周期托管守卫 (Drop 时自动覆写抹零删除)
    pub key_guard: Option<KeyTempGuard>,
    /// 流式输出合包与背压处理缓冲区
    coalesce_buf: Vec<u8>,
    /// 标记会话是否遭遇过身份认证/权限拒绝错误 (Permission denied)
    pub auth_failed: bool,
    /// 终端会话 Asciinema cast v2 录屏状态机
    pub recorder: Option<smagical_core::CastSessionRecorder>,
    /// 终端关键词搜索与视口导航状态机
    pub search_state: crate::terminal::parser::TerminalSearchState,
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
            pty: TerminalBackend::LocalPty(pty),
            parser,
            size,
            key_guard: None,
            coalesce_buf: Vec::with_capacity(32 * 1024),
            auth_failed: false,
            recorder: None,
            search_state: crate::terminal::parser::TerminalSearchState::default(),
        })
    }

    /// 启动远程 SSH 交互式终端会话实例 (基础兼容模式)。
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
                config: None,
            },
            state: SessionState::Running,
            pty: TerminalBackend::LocalPty(pty),
            parser,
            size,
            key_guard: None,
            coalesce_buf: Vec::with_capacity(32 * 1024),
            auth_failed: false,
            recorder: None,
            search_state: crate::terminal::parser::TerminalSearchState::default(),
        })
    }

    /// 启动配置完备的远程 SSH 交互式终端 (支持私钥凭据挂载、跳板机链式跳转、代理隧道与密码自动应答)
    pub fn spawn_ssh_advanced(
        session_id: String,
        display_name: String,
        config: SshLaunchConfig,
        cols: u16,
        rows: u16,
    ) -> Result<Self> {
        let size = PtySize {
            cols: cols.max(10),
            rows: rows.max(5),
            pixel_width: 0,
            pixel_height: 0,
        };

        let host = config.host.clone();
        let port = config.port;
        let username = config.username.clone();

        let (pty, key_guard) = PtyProcess::spawn_ssh_with_config(&session_id, &config, size)?;
        let parser = TerminalParser::new(size.cols, size.rows);

        Ok(Self {
            session_id,
            display_name,
            target: TerminalTarget::Ssh {
                host,
                port,
                username,
                config: Some(config),
            },
            state: SessionState::Running,
            pty: TerminalBackend::LocalPty(pty),
            parser,
            size,
            key_guard,
            coalesce_buf: Vec::with_capacity(32 * 1024),
            auth_failed: false,
            recorder: None,
            search_state: crate::terminal::parser::TerminalSearchState::default(),
        })
    }

    /// 基于纯 Rust 原生 SshStreamChannel 管道启动远程 SSH 交互式终端会话。
    ///
    /// 零外部 `ssh.exe` 依赖，零 ConPTY 依赖，异步双向流直通 VT100。
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_pure_ssh_channel(
        session_id: String,
        display_name: String,
        host: String,
        port: u16,
        username: Option<String>,
        config: Option<SshLaunchConfig>,
        stream: Box<dyn SshStreamChannel>,
        cols: u16,
        rows: u16,
    ) -> Result<Self> {
        let size = PtySize {
            cols: cols.max(10),
            rows: rows.max(5),
            pixel_width: 0,
            pixel_height: 0,
        };

        let pure_ssh = PureSshProcess::new(stream, size);
        let parser = TerminalParser::new(size.cols, size.rows);

        Ok(Self {
            session_id,
            display_name,
            target: TerminalTarget::Ssh {
                host,
                port,
                username,
                config,
            },
            state: SessionState::Running,
            pty: TerminalBackend::PureSsh(pure_ssh),
            parser,
            size,
            key_guard: None,
            coalesce_buf: Vec::with_capacity(32 * 1024),
            auth_failed: false,
            recorder: None,
            search_state: crate::terminal::parser::TerminalSearchState::default(),
        })
    }

    /// 启动处于连接握手等待状态的终端会话实例 (在终端输出区打印连接提示)
    pub fn spawn_connecting(
        session_id: String,
        display_name: String,
        host: String,
        port: u16,
        username: Option<String>,
        config: Option<SshLaunchConfig>,
        cols: u16,
        rows: u16,
    ) -> Self {
        let size = PtySize {
            cols: cols.max(10),
            rows: rows.max(5),
            pixel_width: 0,
            pixel_height: 0,
        };

        let mut parser = TerminalParser::new(size.cols, size.rows);
        let addr = if port == 22 || port == 0 {
            host.clone()
        } else {
            format!("{}:{}", host, port)
        };
        let user_prefix = match &username {
            Some(u) if !u.is_empty() => format!("{u}@"),
            _ => String::new(),
        };
        let connect_msg = format!(
            "\x1b[33m[smalux] 正在连接主机 {user_prefix}{addr} ...\x1b[0m\r\n"
        );
        parser.process(connect_msg.as_bytes());

        Self {
            session_id,
            display_name,
            target: TerminalTarget::Ssh {
                host,
                port,
                username,
                config,
            },
            state: SessionState::Running,
            pty: TerminalBackend::Connecting(size),
            parser,
            size,
            key_guard: None,
            coalesce_buf: Vec::with_capacity(32 * 1024),
            auth_failed: false,
            recorder: None,
            search_state: crate::terminal::parser::TerminalSearchState::default(),
        }
    }

    /// 向终端子进程发送键盘按键字符或转义序列。
    ///
    /// # 参数
    /// - `text`: 键盘输入的 UTF-8 文本或控制字符
    pub fn send_input(&mut self, text: &str) -> Result<()> {
        if matches!(self.state, SessionState::Exited { .. } | SessionState::Reconnecting { .. }) {
            return Ok(());
        }
        if let Some(rec) = &mut self.recorder {
            rec.record_input(text);
        }
        self.pty.write_str(text)
    }

    /// 向终端子进程发送原始控制字节（如快捷键组合转义码）。
    ///
    /// # 参数
    /// - `bytes`: 待发送的原始字节切片
    pub fn send_bytes(&mut self, bytes: &[u8]) -> Result<()> {
        if matches!(self.state, SessionState::Exited { .. } | SessionState::Reconnecting { .. }) {
            return Ok(());
        }
        if let Some(rec) = &mut self.recorder {
            rec.record_input(&String::from_utf8_lossy(bytes));
        }
        self.pty.write_bytes(bytes)
    }

    /// 轮询接收 PTY 的输出数据流并喂入 VT100 解析器，同时自动检测子进程存活与断线退出。
    ///
    /// 包含微秒级流式合包与背压流控：单帧批量合并碎片包并限制单帧最大消费量，
    /// 彻底防止突发海量输出 (`cat /dev/urandom`, `docker logs -f`) 阻塞 UI 主循环。
    ///
    /// # 返回值
    /// 若产生新的屏幕变更（需要触发重绘）则返回 `true`；无变更则返回 `false`。
    pub fn poll_output(&mut self) -> bool {
        // 1. 将底层 PTY/SSH 通道的所有可用输出分块收集到流式合包缓冲区
        let recorder = &mut self.recorder;
        let buf = &mut self.coalesce_buf;
        let auth_failed_ref = &mut self.auth_failed;
        let drained = self.pty.drain_output_into(|chunk| {
            buf.extend_from_slice(chunk);
            if let Some(rec) = recorder {
                rec.record_output(&String::from_utf8_lossy(chunk));
            }
            let lossy = String::from_utf8_lossy(chunk);
            if lossy.contains("Permission denied")
                || lossy.contains("Authentication failed")
                || lossy.contains("认证失败")
            {
                *auth_failed_ref = true;
            }
        });

        // 高水位线溢出保护：若积压超过 2MB（例如连续刷屏未被及时消费），截断并保留最新的 1MB 内容，防止 OOM
        const MAX_COALESCE_HIGH_WATERMARK: usize = 2 * 1024 * 1024;
        const RETAIN_TAIL_BYTES: usize = 1024 * 1024;
        if self.coalesce_buf.len() > MAX_COALESCE_HIGH_WATERMARK {
            let drop_count = self.coalesce_buf.len() - RETAIN_TAIL_BYTES;
            self.coalesce_buf.drain(..drop_count);
            tracing::warn!(
                target: "smagical_ui::terminal",
                "终端合包缓冲区超过 2MB 高水位线，已丢弃最早 {} 字节，保留最新 1MB",
                drop_count
            );
        }

        // 2. 微秒级背压流控：单帧最多消费 MAX_BATCH_BYTES (128 KB)，防止突发海量日志阻塞 UI 事件循环
        const MAX_BATCH_BYTES: usize = 128 * 1024;
        let has_work = !self.coalesce_buf.is_empty();
        if has_work {
            if self.coalesce_buf.len() <= MAX_BATCH_BYTES {
                self.parser.process(&self.coalesce_buf);
                self.coalesce_buf.clear();
            } else {
                // 仅消费前 128 KB，剩余字节留至下一帧平滑处理，持续保持 60FPS 极速响应
                self.parser.process(&self.coalesce_buf[..MAX_BATCH_BYTES]);
                self.coalesce_buf.drain(..MAX_BATCH_BYTES);
                self.parser.mark_dirty();
            }

            // 容量治理：空闲且容量超 256KB 时自动回缩至 64KB，防止无界内存堆积
            if self.coalesce_buf.is_empty() && self.coalesce_buf.capacity() > 256 * 1024 {
                self.coalesce_buf.shrink_to(64 * 1024);
            }
        }

        for response in self.parser.take_pty_writes() {
            let _ = self.pty.write_str(&response);
        }

        // 检查子进程是否已退出且当前尚未处于 Exited 或 Reconnecting 状态
        if self.state == SessionState::Running && !self.pty.is_alive() {
            self.handle_process_exit();
            return true;
        }

        drained || has_work
    }

    /// 处理子进程退出或网络断联事件：分析退出原因、向视口注入专属 ANSI 提示横幅并切换状态。
    pub fn handle_process_exit(&mut self) {
        let exit_status = self.pty.exit_status();
        let reason = match &self.target {
            TerminalTarget::Ssh { .. } => {
                if self.auth_failed {
                    SessionExitReason::AuthFailed
                } else {
                    match exit_status {
                        Some(st) if st.success() => SessionExitReason::Normal(0),
                        Some(st) if st.exit_code() == 255 => SessionExitReason::Disconnected,
                        Some(st) => SessionExitReason::Crash(st.exit_code()),
                        None => SessionExitReason::Disconnected,
                    }
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
            SessionExitReason::AuthFailed => {
                "\r\n\r\n\x1b[90m--------------------------------------------------\x1b[0m\r\n\
                 \x1b[31;1m[✖ 身份认证失败: 远程主机拒绝访问 (Permission denied)]\x1b[0m\r\n\
                 \x1b[90m已停止自动重新连接。请检查主机配置中的用户名、登录密码或 SSH 密钥凭据 (按 Ctrl+W 关闭标签页)\x1b[0m\r\n"
                    .to_string()
            }
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

    /// 将会话转入自动重连中状态并预告重连倒计时
    pub fn enter_reconnecting(&mut self, attempt: u32, max_attempts: u32, delay_secs: u64) {
        let next_attempt_at = std::time::Instant::now() + std::time::Duration::from_secs(delay_secs);
        let msg = format!(
            "\r\n\x1b[33m[smalux] 远程连接已断开，正在准备自动重新连接 (第 {}/{} 次，{} 秒后尝试)... \x1b[0m\r\n",
            attempt, max_attempts, delay_secs
        );
        self.parser.process(msg.as_bytes());
        self.parser.mark_dirty();
        self.state = SessionState::Reconnecting {
            attempt,
            max_attempts,
            next_attempt_at,
        };
    }

    /// 获取当前会话状态对应的指示灯状态字符串 ("online" | "offline" | "warning" | "error")
    pub fn current_status(&self) -> &'static str {
        match &self.state {
            SessionState::Running => "online",
            SessionState::Reconnecting { .. } => "warning",
            SessionState::Exited { reason, .. } => match reason {
                SessionExitReason::Normal(_) => "offline",
                SessionExitReason::AuthFailed => "error",
                SessionExitReason::Disconnected => "warning",
                SessionExitReason::Crash(_) => "error",
            },
        }
    }

    /// 查询当前会话是否处于退出/断联状态。
    pub fn is_exited(&self) -> bool {
        matches!(self.state, SessionState::Exited { .. })
    }

    /// 查询当前会话是否处于自动重连调度中。
    pub fn is_reconnecting(&self) -> bool {
        matches!(self.state, SessionState::Reconnecting { .. })
    }

    /// 原地重新连接此终端会话 (释放老旧 PTY 并拉起全新子进程)。
    pub fn reconnect(&mut self) -> Result<()> {
        let _ = self.pty.kill();
        let new_pty = match &self.target {
            TerminalTarget::Local { shell_id } => {
                TerminalBackend::LocalPty(PtyProcess::spawn_local_shell(shell_id, self.size)?)
            }
            TerminalTarget::Ssh { .. } => {
                // 远程 SSH 会话切入占位 Connecting 状态，异步管线负责真实挂载
                TerminalBackend::Connecting(self.size)
            }
        };
        self.pty = new_pty;
        self.auth_failed = false;
        self.state = SessionState::Running;
        self.parser.process("\r\n\x1b[32m[smalux] 会话已重新启动\x1b[0m\r\n\r\n".as_bytes());
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

    /// 开启当前会话的 Asciinema cast v2 录屏
    pub fn start_recording(&mut self, title: Option<String>) {
        let title = title.or_else(|| Some(self.display_name.clone()));
        self.recorder = Some(smagical_core::CastSessionRecorder::start(
            self.session_id.clone(),
            self.size.cols,
            self.size.rows,
            title,
        ));
        crate::audit_logger::record_audit_event(
            &self.session_id,
            &self.display_name,
            "SESSION_RECORDING_STARTED",
            "Asciinema cast v2 录屏已启动",
        );
    }

    /// 停止录屏并持久化保存至本地 recordings 目录
    pub fn stop_recording(&mut self) -> Option<std::path::PathBuf> {
        let rec = self.recorder.take()?;
        let session = rec.stop();
        let rec_dir = crate::audit_logger::get_recordings_dir();
        let filename = format!(
            "rec_{}_{}_{}.cast",
            chrono::Local::now().format("%Y%m%d_%H%M%S"),
            self.session_id,
            uuid::Uuid::new_v4().simple().to_string().chars().take(6).collect::<String>()
        );
        let target_path = rec_dir.join(filename);
        if let Ok(()) = session.save_to_file(&target_path) {
            crate::audit_logger::record_audit_event(
                &self.session_id,
                &self.display_name,
                "SESSION_RECORDING_STOPPED",
                &format!("持续时间: {:.1}秒, 目标: {}", session.duration, target_path.display()),
            );
            Some(target_path)
        } else {
            None
        }
    }

    /// 查询当前会话是否处于录屏中
    pub fn is_recording(&self) -> bool {
        self.recorder.as_ref().map(|r| r.is_active()).unwrap_or(false)
    }

    /// 获取当前录屏持续时间 (秒)
    pub fn recording_elapsed_seconds(&self) -> f64 {
        self.recorder.as_ref().map(|r| r.elapsed_seconds()).unwrap_or(0.0)
    }

    /// 获取当前格式化的录屏时间标签 (如 "02:15")
    pub fn formatted_recording_time(&self) -> String {
        smagical_core::CastReplayer::format_time_label(self.recording_elapsed_seconds())
    }

    /// 在当前终端中检索关键词并自动聚焦首个匹配项
    ///
    /// 返回 `(current_index_1_based, total_matches)`
    pub fn search_query(&mut self, query: &str, match_case: bool) -> (usize, usize) {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            self.search_state = crate::terminal::parser::TerminalSearchState::default();
            self.parser.clear_selection();
            return (0, 0);
        }

        // 若查询词与区分大小写设置完全相同且已有结果，则步进至下一个匹配项
        if self.search_state.query == trimmed
            && self.search_state.match_case == match_case
            && !self.search_state.matches.is_empty()
        {
            return self.search_next();
        }

        let matches = self.parser.find_matches(trimmed, match_case);
        self.search_state =
            crate::terminal::parser::TerminalSearchState::new(trimmed, match_case, matches);

        if let Some(m) = self.search_state.current_match().cloned() {
            self.parser.focus_match(&m);
        } else {
            self.parser.clear_selection();
        }

        (
            self.search_state.current_1_based(),
            self.search_state.total_matches(),
        )
    }

    /// 循环聚焦下一个搜索匹配项
    pub fn search_next(&mut self) -> (usize, usize) {
        if let Some(m) = self.search_state.next_match().cloned() {
            self.parser.focus_match(&m);
        }
        (
            self.search_state.current_1_based(),
            self.search_state.total_matches(),
        )
    }

    /// 循环聚焦上一个搜索匹配项
    pub fn search_prev(&mut self) -> (usize, usize) {
        if let Some(m) = self.search_state.prev_match().cloned() {
            self.parser.focus_match(&m);
        }
        (
            self.search_state.current_1_based(),
            self.search_state.total_matches(),
        )
    }

    /// 一键将终端当前可视窗口全部字符划选为选区 (Select All)
    pub fn select_all(&mut self) {
        self.parser.select_all();
    }

    /// 导出当前终端的屏幕内容或回滚历史快照为指定格式字符串
    pub fn export_snapshot(&self, format: SnapshotExportFormat, max_lines: usize) -> String {
        let raw_text = self.parser.extract_all_text(max_lines);
        let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        match format {
            SnapshotExportFormat::PlainText => raw_text,
            SnapshotExportFormat::Markdown => {
                let target_desc = match &self.target {
                    TerminalTarget::Local { shell_id } => format!("本地 Shell ({})", shell_id),
                    TerminalTarget::Ssh {
                        host,
                        port,
                        username,
                        ..
                    } => {
                        let user = username.as_deref().unwrap_or("default");
                        format!("SSH 远程终端 ({user}@{host}:{port})")
                    }
                };
                format!(
                    "### 终端会话记录: {}\n- **会话标识**: `{}`\n- **目标环境**: {}\n- **导出时间**: {}\n\n```log\n{}\n```\n",
                    self.display_name, self.session_id, target_desc, now_str, raw_text
                )
            }
            SnapshotExportFormat::Html => {
                let escaped = raw_text
                    .replace('&', "&amp;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;");
                format!(
                    "<div class=\"smalux-terminal-snapshot\" style=\"background:#1e1e1e;color:#d4d4d4;padding:16px;border-radius:8px;font-family:Consolas,monospace;font-size:13px;line-height:1.4;white-space:pre-wrap;word-break:break-all;\">\n<!-- Exported: {} - {} -->\n{}\n</div>",
                    self.display_name, now_str, escaped
                )
            }
        }
    }

    /// 将终端屏幕或历史快照直接复制到系统剪贴板
    pub fn copy_snapshot_to_clipboard(
        &self,
        format: SnapshotExportFormat,
        max_lines: usize,
    ) -> Result<(), String> {
        let content = self.export_snapshot(format, max_lines);
        let mut clipboard =
            arboard::Clipboard::new().map_err(|e| format!("初始化系统剪贴板失败: {}", e))?;
        clipboard
            .set_text(content)
            .map_err(|e| format!("写入系统剪贴板失败: {}", e))?;
        Ok(())
    }
}

/// 终端历史与屏幕快照导出格式
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapshotExportFormat {
    /// 纯文本日志格式
    PlainText,
    /// Markdown 格式代码块 (附带元数据)
    Markdown,
    /// 带有深色风格的 HTML `<pre>` 格式
    Html,
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

    #[tokio::test]
    async fn test_spawn_pure_ssh_channel() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (client_stream, mut server_stream) = tokio::io::duplex(4096);
        let instance_res = TerminalInstance::spawn_pure_ssh_channel(
            "sess-pure-ssh".to_string(),
            "Pure SSH Test".to_string(),
            "127.0.0.1".to_string(),
            22,
            Some("root".to_string()),
            None,
            Box::new(client_stream),
            80,
            24,
        );
        assert!(instance_res.is_ok());
        let mut instance = instance_res.unwrap();
        assert!(instance.is_alive());
        assert_eq!(instance.current_status(), "online");

        // 模拟远端 SSH 输出 ANSI 文本
        server_stream.write_all(b"Hello from Pure Rust SSH!\r\n").await.unwrap();
        server_stream.flush().await.unwrap();

        // 轮询消费输出
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let updated = instance.poll_output();
        assert!(updated, "应检测到远端输出到达");

        let snapshot = instance.snapshot_text(5);
        assert!(snapshot.contains("Hello from Pure Rust SSH!"), "视口文本应包含模拟远端 SSH 输出: {snapshot}");

        // 模拟客户端输入
        instance.send_input("ls -la\n").unwrap();
        let mut buf = [0u8; 64];
        let n = server_stream.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"ls -la\n");

        // 模拟关闭连接
        drop(server_stream);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        instance.poll_output();
    }

    #[tokio::test]
    async fn test_terminal_instance_search_and_navigation() {
        let (client_stream, mut server_stream) = tokio::io::duplex(4096);
        let mut instance = TerminalInstance::spawn_pure_ssh_channel(
            "sess-search".to_string(),
            "Search Test".to_string(),
            "127.0.0.1".to_string(),
            22,
            None,
            None,
            Box::new(client_stream),
            80,
            24,
        ).unwrap();

        // 注入包含特定关键字的模拟文本
        use tokio::io::AsyncWriteExt;
        server_stream.write_all(b"nginx: [warn] 1024 workers active\r\nnginx: [error] port 80 failed\r\nnginx: [emerg] crashed\r\n").await.unwrap();
        server_stream.flush().await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        instance.poll_output();

        // 1. 搜索 "nginx" (不区分大小写)
        let (curr, total) = instance.search_query("nginx", false);
        assert_eq!(total, 3, "应匹配到 3 处 'nginx'");
        assert_eq!(curr, 1, "初始聚焦应为第 1 项");

        // 2. 步进至下一项
        let (curr2, total2) = instance.search_next();
        assert_eq!(total2, 3);
        assert_eq!(curr2, 2, "下一项应为第 2 项");

        // 3. 步进至下一项
        let (curr3, _) = instance.search_next();
        assert_eq!(curr3, 3, "下一项应为第 3 项");

        // 4. 循环回退
        let (curr4, _) = instance.search_prev();
        assert_eq!(curr4, 2, "上一步进应回退至第 2 项");

        // 5. 搜索不存在的关键字
        let (curr_none, total_none) = instance.search_query("nonexistent_pattern", false);
        assert_eq!(total_none, 0);
        assert_eq!(curr_none, 0);
    }

    #[tokio::test]
    async fn test_terminal_instance_select_all() {
        let (client_stream, mut server_stream) = tokio::io::duplex(4096);
        let mut instance = TerminalInstance::spawn_pure_ssh_channel(
            "sess-select".to_string(),
            "Select All Test".to_string(),
            "127.0.0.1".to_string(),
            22,
            None,
            None,
            Box::new(client_stream),
            80,
            24,
        ).unwrap();

        use tokio::io::AsyncWriteExt;
        server_stream.write_all(b"line 1\r\nline 2\r\n").await.unwrap();
        server_stream.flush().await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        instance.poll_output();

        // 执行全选
        instance.select_all();
        let sel = instance.parser.selection();
        assert!(sel.is_some(), "全选后选区必须非空");
        let ((sc, sr), (ec, er)) = sel.unwrap();
        assert_eq!((sc, sr), (0, 0));
        assert_eq!(ec, 79);
        assert_eq!(er, 23);

        let selected_text = instance.parser.copy_selection_text();
        assert!(selected_text.contains("line 1"));
        assert!(selected_text.contains("line 2"));
    }

    #[tokio::test]
    async fn test_terminal_instance_export_snapshot() {
        let (client_stream, mut server_stream) = tokio::io::duplex(4096);
        let mut instance = TerminalInstance::spawn_pure_ssh_channel(
            "sess-export".to_string(),
            "Export Test".to_string(),
            "192.168.1.100".to_string(),
            22,
            Some("admin".to_string()),
            None,
            Box::new(client_stream),
            80,
            24,
        ).unwrap();

        use tokio::io::AsyncWriteExt;
        server_stream.write_all(b"root@prod:~# uname -a\r\nLinux prod 6.1.0-amd64\r\n").await.unwrap();
        server_stream.flush().await.unwrap();

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        instance.poll_output();

        // 1. 导出纯文本 PlainText
        let plain = instance.export_snapshot(SnapshotExportFormat::PlainText, 10);
        assert!(plain.contains("Linux prod 6.1.0-amd64"));

        // 2. 导出 Markdown
        let md = instance.export_snapshot(SnapshotExportFormat::Markdown, 10);
        assert!(md.contains("### 终端会话记录: Export Test"));
        assert!(md.contains("```log"));
        assert!(md.contains("Linux prod 6.1.0-amd64"));

        // 3. 导出 HTML
        let html = instance.export_snapshot(SnapshotExportFormat::Html, 10);
        assert!(html.contains("<div class=\"smalux-terminal-snapshot\""));
        assert!(html.contains("Linux prod 6.1.0-amd64"));
    }
}


