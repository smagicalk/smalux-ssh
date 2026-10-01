//! # Smalux CLI - 纯 Rust 原生交互式 SSH 终端会话桥接器
//!
//! 在操作系统终端中直接分配并接管 PTY 交互，通过双向字节流桥接本地终端标准输入/输出与远端虚拟控制台，
//! 提供与 OpenSSH `ssh user@host` 100% 一致的操作体验，零外部依赖。

use std::sync::Arc;
use anyhow::{Context, Result};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use smagical_core::domain::{CredentialRecord, HostRecord};
use smagical_core::service::SshSessionService;
use smagical_ssh::RusshSessionDriver;

/// RAII 终端原始模式守卫，确保无论正常退出还是异常退出均自动复原控制台模式
struct RawModeGuard;

impl RawModeGuard {
    fn new() -> Result<Self> {
        enable_raw_mode().context("启用终端 Raw Mode 失败")?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

/// 运行纯 Rust 原生交互式 SSH 终端会话
pub async fn run_interactive_session(
    host: &HostRecord,
    cred: Option<&CredentialRecord>,
    ssh_svc: &Arc<RusshSessionDriver>,
) -> Result<()> {
    println!(
        "正在通过纯 Rust 原生协议引擎连接 {} ({}:{})...",
        host.name, host.address, host.port
    );

    let session_id = ssh_svc
        .connect(host, cred)
        .await
        .map_err(|e| anyhow::anyhow!("建立 SSH 会话失败: {e}"))?;

    let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
    let term_type = host.term_type.as_deref().unwrap_or("xterm-256color");


    let pty_channel = ssh_svc
        .open_pty_channel(&session_id, term_type, rows, cols)
        .await
        .map_err(|e| anyhow::anyhow!("分配远端虚拟终端 PTY 失败: {e}"))?;

    // 进入原始模式，接管字符流
    let _guard = RawModeGuard::new()?;

    let (mut pty_read, mut pty_write) = tokio::io::split(pty_channel);
    let mut stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();

    let mut in_buf = [0u8; 1024];
    let mut out_buf = [0u8; 4096];

    loop {
        tokio::select! {
            // 本地键盘输入 -> 转发给远端 SSH PTY
            res = stdin.read(&mut in_buf) => {
                match res {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if pty_write.write_all(&in_buf[..n]).await.is_err() {
                            break;
                        }
                        let _ = pty_write.flush().await;
                    }
                    Err(_) => break,
                }
            }
            // 远端 SSH PTY 回显 -> 写入本地终端控制台
            res = pty_read.read(&mut out_buf) => {
                match res {
                    Ok(0) => break, // 远端会话关闭 (如输入 exit)
                    Ok(n) => {
                        if stdout.write_all(&out_buf[..n]).await.is_err() {
                            break;
                        }
                        let _ = stdout.flush().await;
                    }
                    Err(_) => break,
                }
            }
        }
    }

    drop(_guard);
    let _ = ssh_svc.disconnect(&session_id).await;
    println!("\r\n[会话已退出连接: {}]\r\n", host.name);

    Ok(())
}
