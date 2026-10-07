//! 纯 Rust 原生 SSH 双向交互式会话驱动管道 (PureSshProcess)。
//!
//! 直接连接 `smagical_core::service::ssh::SshStreamChannel` 双向异步流，
//! 完全摆脱对系统底层 `ssh.exe` 进程及 ConPTY 的依赖：
//! 1. 纯异步非阻塞 Tokio 读写管道分拆 (split)；
//! 2. 跨线程安全：输入端由异步 Channel 消费，输出端由同步 MPSC 喂入 VT100 解析器；
//! 3. 原生生命周期管理与优雅关闭 (Drop/Kill 抹除)。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use anyhow::{Context, Result};
use smagical_core::service::ssh::SshStreamChannel;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::task::JoinHandle;

use crate::terminal::pty::PtySize;

/// 纯 Rust 原生 SSH 交互会话管道实体
pub struct PureSshProcess {
    tx_input: UnboundedSender<Vec<u8>>,
    rx_output: Receiver<Vec<u8>>,
    is_alive: Arc<AtomicBool>,
    tasks: Vec<JoinHandle<()>>,
    size: PtySize,
}

impl PureSshProcess {
    /// 基于异步双向流初始化纯 Rust 交互式终端会话
    pub fn new(stream: Box<dyn SshStreamChannel>, size: PtySize) -> Self {
        let (tx_input, mut rx_input) = unbounded_channel::<Vec<u8>>();
        let (tx_output, rx_output) = channel::<Vec<u8>>();
        let is_alive = Arc::new(AtomicBool::new(true));

        let (mut read_half, mut write_half) = tokio::io::split(stream);

        // 1. 异步读取协程：将远端 SSH 流读取的数据同步传输给前端 VT100
        let is_alive_read = Arc::clone(&is_alive);
        let tx_out_clone = tx_output;
        let read_task = tokio::spawn(async move {
            let mut buf = [0u8; 8192];
            use tokio::io::AsyncReadExt;
            while is_alive_read.load(Ordering::Relaxed) {
                match read_half.read(&mut buf).await {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if tx_out_clone.send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::debug!("原生 SSH 流读取终止: {e}");
                        break;
                    }
                }
            }
            is_alive_read.store(false, Ordering::Relaxed);
        });

        // 2. 异步写入协程：将键盘输入发送给远端 SSH
        let is_alive_write = Arc::clone(&is_alive);
        let write_task = tokio::spawn(async move {
            use tokio::io::AsyncWriteExt;
            while let Some(bytes) = rx_input.recv().await {
                if !is_alive_write.load(Ordering::Relaxed) {
                    break;
                }
                if let Err(e) = write_half.write_all(&bytes).await {
                    tracing::debug!("原生 SSH 流写入异常: {e}");
                    break;
                }
                let _ = write_half.flush().await;
            }
            let _ = write_half.shutdown().await;
            is_alive_write.store(false, Ordering::Relaxed);
        });

        Self {
            tx_input,
            rx_output,
            is_alive,
            tasks: vec![read_task, write_task],
            size,
        }
    }

    /// 向 SSH 终端发送原始输入字节流
    pub fn write_bytes(&self, bytes: &[u8]) -> Result<()> {
        self.tx_input
            .send(bytes.to_vec())
            .context("向原生 SSH 输入管道写入数据失败")
    }

    /// 向 SSH 终端发送 UTF-8 文本字符串
    pub fn write_str(&self, text: &str) -> Result<()> {
        self.write_bytes(text.as_bytes())
    }

    /// 非阻塞尝试读取当前所有已到达的输出字节
    pub fn try_recv_output(&self) -> Vec<Vec<u8>> {
        let mut chunks = Vec::new();
        while let Ok(chunk) = self.rx_output.try_recv() {
            chunks.push(chunk);
        }
        chunks
    }

    /// 高效消费当前累积的全部输出字节块直接喂入闭包，避免分配中间 Vec<Vec<u8>>
    pub fn drain_output_into<F: FnMut(&[u8])>(&self, mut consumer: F) -> bool {
        let mut had_any = false;
        while let Ok(chunk) = self.rx_output.try_recv() {
            had_any = true;
            consumer(&chunk);
        }
        had_any
    }

    /// 动态更新终端视口尺寸
    pub fn resize(&mut self, size: PtySize) -> Result<()> {
        self.size = size;
        Ok(())
    }

    /// 查询会话是否仍在活跃运行
    pub fn is_alive(&mut self) -> bool {
        self.is_alive.load(Ordering::Relaxed)
    }

    /// 获取退出状态 (模拟返回)
    pub fn exit_status(&mut self) -> Option<portable_pty::ExitStatus> {
        if !self.is_alive() {
            Some(portable_pty::ExitStatus::with_exit_code(0))
        } else {
            None
        }
    }

    /// 终止会话并终止后台任务
    pub fn kill(&mut self) -> Result<()> {
        self.is_alive.store(false, Ordering::Relaxed);
        for task in &self.tasks {
            task.abort();
        }
        Ok(())
    }

    /// 获取当前生效的几何尺寸
    pub fn size(&self) -> PtySize {
        self.size
    }
}

impl Drop for PureSshProcess {
    fn drop(&mut self) {
        let _ = self.kill();
    }
}
