//! 终端底层驱动抽象 (TerminalBackend)。
//!
//! 统一抹平本地 ConPTY 子进程与纯 Rust 原生 SSH 双向流通道的差异：
//! - `LocalPty`: 封装系统本地子进程 PTY
//! - `PureSsh`: 纯 Rust 原生 SshStreamChannel 双向异步管道

use anyhow::Result;
use crate::terminal::pty::{PtyProcess, PtySize};
use crate::terminal::pure_ssh::PureSshProcess;

/// 终端底层会话后端
pub enum TerminalBackend {
    /// 本地 PTY 伪终端进程
    LocalPty(PtyProcess),
    /// 纯 Rust 原生 SSH 双向流
    PureSsh(PureSshProcess),
    /// 正在后台握手建立连接中 (占位等待真实数据流挂载)
    Connecting(PtySize),
}

impl TerminalBackend {
    /// 向终端后端写入原始字节流
    pub fn write_bytes(&self, bytes: &[u8]) -> Result<()> {
        match self {
            Self::LocalPty(p) => p.write_bytes(bytes),
            Self::PureSsh(p) => p.write_bytes(bytes),
            Self::Connecting(_) => Ok(()),
        }
    }

    /// 向终端后端写入 UTF-8 文本
    pub fn write_str(&self, text: &str) -> Result<()> {
        match self {
            Self::LocalPty(p) => p.write_str(text),
            Self::PureSsh(p) => p.write_str(text),
            Self::Connecting(_) => Ok(()),
        }
    }

    /// 轮询获取输出字节块集合
    pub fn try_recv_output(&self) -> Vec<Vec<u8>> {
        match self {
            Self::LocalPty(p) => p.try_recv_output(),
            Self::PureSsh(p) => p.try_recv_output(),
            Self::Connecting(_) => Vec::new(),
        }
    }

    /// 调整视口几何尺寸
    pub fn resize(&mut self, size: PtySize) -> Result<()> {
        match self {
            Self::LocalPty(p) => p.resize(size),
            Self::PureSsh(p) => p.resize(size),
            Self::Connecting(s) => {
                *s = size;
                Ok(())
            }
        }
    }

    /// 查询后端是否仍在活跃运行
    pub fn is_alive(&mut self) -> bool {
        match self {
            Self::LocalPty(p) => p.is_alive(),
            Self::PureSsh(p) => p.is_alive(),
            Self::Connecting(_) => true,
        }
    }

    /// 获取退出状态
    pub fn exit_status(&mut self) -> Option<portable_pty::ExitStatus> {
        match self {
            Self::LocalPty(p) => p.exit_status(),
            Self::PureSsh(p) => p.exit_status(),
            Self::Connecting(_) => None,
        }
    }

    /// 强制终止后端会话
    pub fn kill(&mut self) -> Result<()> {
        match self {
            Self::LocalPty(p) => p.kill(),
            Self::PureSsh(p) => p.kill(),
            Self::Connecting(_) => Ok(()),
        }
    }

    /// 获取当前行列尺寸
    pub fn size(&self) -> PtySize {
        match self {
            Self::LocalPty(p) => p.size(),
            Self::PureSsh(p) => p.size(),
            Self::Connecting(s) => *s,
        }
    }
}
