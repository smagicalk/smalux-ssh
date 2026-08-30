//! 本地 PTY 伪终端进程托管与生命周期管理。
//!
//! 跨平台托管本地子进程（Windows ConPTY、Linux/macOS Unix PTY），处理非阻塞 I/O 流转发与动态网格尺寸伸缩。

use std::io::{Read, Write};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize as PortablePtySize};

/// 终端视口网格与像素几何尺寸定义。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PtySize {
    /// 终端可见字符列数 (Columns)
    pub cols: u16,
    /// 终端可见字符行数 (Rows)
    pub rows: u16,
    /// 视口实际像素宽度 (Pixel Width)
    pub pixel_width: u16,
    /// 视口实际像素高度 (Pixel Height)
    pub pixel_height: u16,
}

impl Default for PtySize {
    fn default() -> Self {
        Self {
            cols: 80,
            rows: 24,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

impl From<PtySize> for PortablePtySize {
    fn from(size: PtySize) -> Self {
        PortablePtySize {
            rows: size.rows,
            cols: size.cols,
            pixel_width: size.pixel_width,
            pixel_height: size.pixel_height,
        }
    }
}

/// 跨平台本地 PTY 进程托管实例。
///
/// 封装底层主从 PTY 对、输入写入流、异步输出读取通道与子进程生命周期句柄。
pub struct PtyProcess {
    /// PTY 主设备控制句柄 (Master PTY)，用于动态下发尺寸变更
    master: Box<dyn MasterPty + Send>,
    /// 标准输入写入句柄
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    /// 异步读取子进程输出字节块的接收通道
    rx_output: Receiver<Vec<u8>>,
    /// 子进程退出状态句柄
    child: Box<dyn Child + Send + Sync>,
    /// 当前生效的终端尺寸
    size: PtySize,
}

impl PtyProcess {
    /// 启动通用命令行进程并初始化 PTY 双向管道。
    ///
    /// # 参数
    /// - `cmd`: 预先配置好的 `CommandBuilder` 启动参数
    /// - `size`: 初始终端视口行列尺寸
    /// - `reader_name`: 专用 I/O 读取线程名称
    ///
    /// # 错误
    /// 若系统 ConPTY/Unix PTY 初始化失败或子进程启动失败，将返回 `anyhow::Result`。
    pub fn spawn_command(
        mut cmd: CommandBuilder,
        size: PtySize,
        reader_name: String,
    ) -> Result<Self> {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(size.into())
            .context("创建原生 PTY 伪终端通道失败")?;

        // 设置默认工作目录为用户主目录
        if let Some(user_home) = directories::UserDirs::new() {
            cmd.cwd(user_home.home_dir());
        }

        // 设置通用终端环境变量
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");

        let child = pair
            .slave
            .spawn_command(cmd)
            .context("启动终端子进程失败")?;

        let writer = pair.master.take_writer().context("获取 PTY 写入流失败")?;
        let mut reader = pair.master.try_clone_reader().context("克隆 PTY 读取流失败")?;

        let (tx_output, rx_output): (Sender<Vec<u8>>, Receiver<Vec<u8>>) = channel();

        // 启动后台专用 I/O 读取线程，持续泵送 PTY 字节流
        thread::Builder::new()
            .name(reader_name)
            .spawn(move || {
                let mut buf = [0u8; 8192];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => {
                            // 子进程已退出或管道 EOF
                            break;
                        }
                        Ok(n) => {
                            if tx_output.send(buf[..n].to_vec()).is_err() {
                                // 接收端通道已关闭
                                break;
                            }
                        }
                        Err(e) => {
                            tracing::debug!(target: "smagical_ui::pty", "PTY 读取线程结束: {:?}", e);
                            break;
                        }
                    }
                }
            })
            .context("创建 PTY 后台读取线程失败")?;

        Ok(Self {
            master: pair.master,
            writer: Arc::new(Mutex::new(writer)),
            rx_output,
            child,
            size,
        })
    }

    /// 启动本地终端 Shell 进程并初始化 PTY 双向管道。
    ///
    /// # 参数
    /// - `shell_id`: 本地终端标识 (如 `"local-pwsh7"`, `"local-powershell"`, `"local-cmd"`, `"local-wsl"`, `"local-bash"`)
    /// - `size`: 初始终端视口行列尺寸
    ///
    /// # 错误
    /// 若系统 ConPTY/Unix PTY 初始化失败或子进程启动失败，将返回 `anyhow::Result`。
    pub fn spawn_local_shell(shell_id: &str, size: PtySize) -> Result<Self> {
        let cmd = Self::resolve_command_by_id(shell_id);
        Self::spawn_command(cmd, size, format!("pty-reader-{}", shell_id))
    }

    /// 启动远程 SSH 伪终端交互进程。
    ///
    /// # 参数
    /// - `host`: 目标远程主机 IPv4/IPv6 或域名
    /// - `port`: SSH 服务监听端口 (通常为 22)
    /// - `username`: 登录用户名 (可选)
    /// - `size`: 初始终端视口行列尺寸
    ///
    /// # 错误
    /// 若无法拉起 SSH 客户端进程或 PTY 创建失败，将返回 `anyhow::Result`。
    pub fn spawn_ssh(
        host: &str,
        port: u16,
        username: Option<&str>,
        size: PtySize,
    ) -> Result<Self> {
        let mut cmd = CommandBuilder::new("ssh");
        cmd.arg("-p");
        cmd.arg(port.to_string());
        if let Some(user) = username {
            cmd.arg(format!("{}@{}", user, host));
        } else {
            cmd.arg(host);
        }
        Self::spawn_command(cmd, size, format!("pty-ssh-{}", host))
    }


    /// 向 PTY 伪终端发送原始输入字节流（键盘敲击、转义序列等）。
    ///
    /// # 参数
    /// - `bytes`: 待写入的字节切片
    pub fn write_bytes(&self, bytes: &[u8]) -> Result<()> {
        let mut w = self.writer.lock().map_err(|_| anyhow::anyhow!("PTY 写入锁已被污染"))?;
        w.write_all(bytes).context("向 PTY 写入数据失败")?;
        w.flush().context("刷新 PTY 写入缓冲区失败")?;
        Ok(())
    }

    /// 向 PTY 发送 UTF-8 文本字符串。
    pub fn write_str(&self, text: &str) -> Result<()> {
        self.write_bytes(text.as_bytes())
    }

    /// 非阻塞尝试读取当前所有已到达的 PTY 输出字节。
    ///
    /// # 返回值
    /// 返回所有累积的输出字节块向量集合；若无新输出则返回空向量。
    pub fn try_recv_output(&self) -> Vec<Vec<u8>> {
        let mut chunks = Vec::new();
        while let Ok(chunk) = self.rx_output.try_recv() {
            chunks.push(chunk);
        }
        chunks
    }

    /// 动态更新终端视口网格行列尺寸（通知子进程 `SIGWINCH` / `ResizePseudoConsole`）。
    ///
    /// # 参数
    /// - `size`: 最新的终端几何行列尺寸
    pub fn resize(&mut self, size: PtySize) -> Result<()> {
        if self.size == size {
            return Ok(());
        }
        self.master
            .resize(size.into())
            .context("调整 PTY 视口尺寸失败")?;
        self.size = size;
        Ok(())
    }

    /// 查询当前终端子进程是否仍在活跃运行。
    pub fn is_alive(&mut self) -> bool {
        match self.child.try_wait() {
            Ok(Some(_status)) => false,
            Ok(None) => true,
            Err(_) => false,
        }
    }

    /// 强制终止子进程生命周期。
    pub fn kill(&mut self) -> Result<()> {
        self.child.kill().context("终止 PTY 子进程失败")?;
        Ok(())
    }

    /// 获取当前生效的终端行列尺寸。
    pub fn size(&self) -> PtySize {
        self.size
    }

    /// 根据本地 Shell 标识构建命令行启动配置。
    fn resolve_command_by_id(shell_id: &str) -> CommandBuilder {
        #[cfg(windows)]
        {
            match shell_id {
                "local-pwsh7" => {
                    let pwsh7_path = "C:\\Program Files\\PowerShell\\7\\pwsh.exe";
                    if std::path::Path::new(pwsh7_path).exists() {
                        CommandBuilder::new(pwsh7_path)
                    } else {
                        CommandBuilder::new("pwsh.exe")
                    }
                }
                "local-powershell" => CommandBuilder::new("powershell.exe"),
                "local-cmd" => CommandBuilder::new("cmd.exe"),
                "local-wsl" => CommandBuilder::new("wsl.exe"),
                "local-gitbash" => {
                    let git_bash = "C:\\Program Files\\Git\\bin\\bash.exe";
                    if std::path::Path::new(git_bash).exists() {
                        CommandBuilder::new(git_bash)
                    } else {
                        CommandBuilder::new("bash.exe")
                    }
                }
                "local-nushell" => CommandBuilder::new("nu.exe"),
                _ => {
                    // 默认降级尝试 PowerShell
                    CommandBuilder::new("powershell.exe")
                }
            }
        }

        #[cfg(not(windows))]
        {
            match shell_id {
                "local-bash" => CommandBuilder::new("/bin/bash"),
                "local-zsh" => CommandBuilder::new("/bin/zsh"),
                "local-fish" => CommandBuilder::new("/usr/bin/fish"),
                "local-sh" => CommandBuilder::new("/bin/sh"),
                "local-nushell" => CommandBuilder::new("/usr/bin/nu"),
                _ => {
                    let default_sh = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
                    CommandBuilder::new(default_sh)
                }
            }
        }
    }
}
