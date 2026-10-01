//! 本地 PTY 伪终端进程托管与生命周期管理。
//!
//! 跨平台托管本地子进程（Windows ConPTY、Linux/macOS Unix PTY），处理非阻塞 I/O 流转发与动态网格尺寸伸缩。

use std::io::{Read, Write};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{Context, Result};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize as PortablePtySize};
use crate::terminal::ssh_config::{KeyTempGuard, SshLaunchConfig};

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
        cmd: CommandBuilder,
        size: PtySize,
        reader_name: String,
    ) -> Result<Self> {
        Self::spawn_command_with_auto_password(cmd, size, reader_name, None)
    }

    /// 启动通用命令行进程并初始化 PTY 双向管道 (支持密码自动应答探测)
    pub fn spawn_command_with_auto_password(
        mut cmd: CommandBuilder,
        size: PtySize,
        reader_name: String,
        auto_password: Option<String>,
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

        let writer_arc = Arc::new(Mutex::new(writer));
        let writer_for_thread = Arc::clone(&writer_arc);

        // 启动后台专用 I/O 读取线程，持续泵送 PTY 字节流并支持前 8 秒密码自动应答
        thread::Builder::new()
            .name(reader_name)
            .spawn(move || {
                let mut buf = [0u8; 8192];
                let mut pwd_opt = auto_password;
                let started_at = std::time::Instant::now();

                let mut sniff_buf = String::new();

                loop {
                    match reader.read(&mut buf) {
                        Ok(0) => {
                            // 子进程已退出或管道 EOF
                            break;
                        }
                        Ok(n) => {
                            let chunk = &buf[..n];

                            // 密码自动应答嗅探 (在启动前 8 秒内有效，支持跨数据分片滑动窗口)
                            if let Some(ref pwd) = pwd_opt {
                                if started_at.elapsed().as_secs() < 8 {
                                    let s = String::from_utf8_lossy(chunk);
                                    sniff_buf.push_str(&s);
                                    if sniff_buf.len() > 1024 {
                                        let trim_start = sniff_buf.len() - 512;
                                        sniff_buf = sniff_buf[trim_start..].to_string();
                                    }
                                    let lower = sniff_buf.to_ascii_lowercase();
                                    if lower.contains("password") || sniff_buf.contains("密码") || lower.contains("passphrase") {
                                        tracing::info!(target: "smagical_ui::pty", "检测到远程密码/口令提示符，正在自动填充密码...");
                                        thread::sleep(std::time::Duration::from_millis(50));
                                        if let Ok(mut w) = writer_for_thread.lock() {
                                            let _ = w.write_all(format!("{}\r\n", pwd).as_bytes());
                                            let _ = w.flush();
                                        }
                                        pwd_opt = None; // 阅后即焚，立刻从内存抹除
                                        sniff_buf.clear();
                                    }
                                } else {
                                    pwd_opt = None; // 超时销毁
                                    sniff_buf.clear();
                                }
                            }

                            if tx_output.send(chunk.to_vec()).is_err() {
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
            writer: writer_arc,
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

    /// 启动远程 SSH 伪终端交互进程 (基础快捷方式)。
    pub fn spawn_ssh(
        host: &str,
        port: u16,
        username: Option<&str>,
        size: PtySize,
    ) -> Result<Self> {
        let config = SshLaunchConfig {
            host: host.to_string(),
            port,
            username: username.map(|s| s.to_string()),
            ..Default::default()
        };
        let (pty, _guard) = Self::spawn_ssh_with_config("default-sess", &config, size)?;
        Ok(pty)
    }

    /// 启动配置完备的远程 SSH 交互式终端 (支持私钥凭据挂载、跳板机链式跳转、代理隧道与自动密码应答)
    pub fn spawn_ssh_with_config(
        session_id: &str,
        config: &SshLaunchConfig,
        size: PtySize,
    ) -> Result<(Self, Option<KeyTempGuard>)> {
        let mut cmd = CommandBuilder::new("ssh");

        // 统一由 smagical-ssh::SshLaunchConfig 构建标准命令行参数与临时私钥守卫
        let (args, key_guard) = config.build_ssh_args(session_id, true)?;
        for arg in args {
            cmd.arg(arg);
        }

        // 目标远程主机 (如 "user@host" 或 "host")
        cmd.arg(config.destination());

        // 启动 PTY 伪终端进程
        let pty = Self::spawn_command_with_auto_password(
            cmd,
            size,
            format!("pty-ssh-{}", config.host),
            config.password.clone(),
        )?;

        Ok((pty, key_guard))
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

    /// 获取子进程退出状态 (若子进程已结束则返回 Some(status)，仍在运行中返回 None)。
    pub fn exit_status(&mut self) -> Option<portable_pty::ExitStatus> {
        self.child.try_wait().unwrap_or_default()
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
    /// 统一委托给 local_shells 模块，使用已探测验证的绝对路径与专用参数 (如 Git Bash 的 --login -i)。
    fn resolve_command_by_id(shell_id: &str) -> CommandBuilder {
        crate::local_shells::resolve_command_for_shell(shell_id)
    }
}
