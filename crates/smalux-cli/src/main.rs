//! # Smalux CLI - 纯 Rust 双模（Headless CLI + TUI 交互）资产与运维终端
//!
//! 基于 `smagical-core` 六边形契约、`smagical-ssh` 纯 Rust 协议引擎与 `smagical-storage`，
//! 提供两种使用模式：
//! 1. **直接运行模式** (Headless CLI): `smalux host list`, `smalux host exec`, `smalux host connect`，适合脚本自动化与快速执行；
//! 2. **交互式 TUI 模式** (Rich Terminal UI): `smalux` 或 `smalux -i`，提供全屏资产列表浏览、搜索、回车直连与主密码弹窗解锁。

pub mod auth;
pub mod terminal_session;
pub mod tui;

use std::sync::Arc;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use smagical_core::service::{KeyAlgorithm, KeygenService, SshSessionService};
use smagical_core::storage::AppStorage;
use smagical_ssh::{NativeKeygenService, RusshSessionDriver};
use smagical_storage::mock::MockStorage;
use smagical_storage::seaorm::SeaOrmStorage;

#[derive(Parser, Debug)]
#[command(name = "smalux")]
#[command(bin_name = "smalux")]
#[command(version, about = "Smalux Headless SSH & Asset CLI / TUI", long_about = None)]
struct Cli {
    /// 显式进入全屏交互式 TUI 仪表盘
    #[arg(short = 'i', long = "tui")]
    tui: bool,

    /// 本地资产保险库主密码 (若开启主密码，可由此参数或 SMALUX_MASTER_PASSWORD 环境变量提供)
    #[arg(short = 'p', long = "master-password")]
    master_password: Option<String>,

    /// 待执行的命令行子命令 (未指定时默认进入 TUI 交互仪表盘模式)
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// 启动全屏终端交互仪表盘 (TUI)
    Tui,
    /// 主机资产管理、直接连接与远程执行
    Host {
        #[command(subcommand)]
        sub: HostCommands,
    },
    /// SSH 密钥对现场生成 (零外部 ssh-keygen 依赖)
    Keygen {
        /// 密钥算法 (ed25519 / rsa / ecdsa)
        #[arg(short, long, default_value = "ed25519")]
        algo: String,
        /// 可选的密钥保护口令
        #[arg(short, long)]
        passphrase: Option<String>,
        /// RSA 密钥位数 (仅 rsa 算法有效，默认 4096)
        #[arg(short, long)]
        bits: Option<u32>,
        /// 输出私钥文件路径 (可选，未提供则仅在终端显示指纹与公钥)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// 网络端口转发与隧道资产配置
    Tunnel {
        #[command(subcommand)]
        sub: TunnelCommands,
    },
    /// 查看版本与纯 Rust 六边形架构引擎信息
    Version,
}

#[derive(Subcommand, Debug)]
enum HostCommands {
    /// 列出所有已录入的主机资产
    List {
        /// 以 JSON 格式输出资产清单 (便于脚本管道与外部系统集成)
        #[arg(long)]
        json: bool,
    },
    /// 建立纯 Rust 原生交互式 SSH 终端连接 (Zero ssh.exe 依赖)
    Connect {
        /// 主机记录 ID
        #[arg(short, long)]
        id: String,
    },
    /// 纯 Rust 原生 SSH 远程单次执行命令 (Zero ssh.exe 依赖)
    Exec {
        /// 主机记录 ID
        #[arg(short, long)]
        id: String,
        /// 待执行的 Bash/Shell 命令
        #[arg(short, long)]
        cmd: String,
    },
}

#[derive(Subcommand, Debug)]
enum TunnelCommands {
    /// 列出所有配置的本地/远程网络隧道
    List,
}

/// 自动探测并初始化存储层 (优先读取本地默认 SQLite 数据库，失败回退 Mock 存储)
async fn init_storage() -> Arc<dyn AppStorage> {
    if let Ok(storage) = SeaOrmStorage::open_default().await {
        return Arc::new(storage);
    }

    // 默认回退至已内建样例数据的 MockStorage
    Arc::new(MockStorage::new())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();
    let storage = init_storage().await;
    let keygen_svc = NativeKeygenService::new();
    let ssh_svc = Arc::new(RusshSessionDriver::new());

    // 判定是否进入全屏 TUI 模式 (显式指定 --tui、未提供子命令或指定 tui 子命令)
    let is_tui_mode = cli.tui || cli.command.is_none() || matches!(cli.command, Some(Commands::Tui));

    if is_tui_mode {
        return tui::run_tui(storage, ssh_svc, cli.master_password.as_deref()).await;
    }

    // 否则执行纯命令行模式
    match cli.command.unwrap() {
        Commands::Tui => unreachable!(),
        Commands::Host { sub } => match sub {
            HostCommands::List { json } => {
                auth::ensure_vault_unlocked(&storage, cli.master_password.as_deref()).await?;
                let hosts = storage.hosts().list_all().await.context("读取主机列表失败")?;

                if json {
                    println!("{}", serde_json::to_string_pretty(&hosts)?);
                } else {
                    println!("{:<16} {:<24} {:<20} {:<8} {:<10}", "ID", "NAME", "ADDRESS", "PORT", "STATUS");
                    println!("{}", "-".repeat(82));
                    for h in hosts {
                        println!(
                            "{:<16} {:<24} {:<20} {:<8} {:<10}",
                            h.id, h.name, h.address, h.port, h.status.as_str()
                        );
                    }
                }
            }
            HostCommands::Connect { id } => {
                auth::ensure_vault_unlocked(&storage, cli.master_password.as_deref()).await?;
                let host = storage
                    .hosts()
                    .get_by_id(&id)
                    .await
                    .context("查询主机记录失败")?
                    .ok_or_else(|| anyhow::anyhow!("未找到 ID 为 [{}] 的主机资产", id))?;

                let cred = if let Some(ref c_id) = host.credential_id {
                    storage.credentials().get_by_id(c_id).await.ok().flatten()
                } else {
                    None
                };

                terminal_session::run_interactive_session(&host, cred.as_ref(), &ssh_svc).await?;
            }
            HostCommands::Exec { id, cmd } => {
                auth::ensure_vault_unlocked(&storage, cli.master_password.as_deref()).await?;
                let host = storage
                    .hosts()
                    .get_by_id(&id)
                    .await
                    .context("查询主机记录失败")?
                    .ok_or_else(|| anyhow::anyhow!("未找到 ID 为 [{}] 的主机资产", id))?;

                let cred = if let Some(ref c_id) = host.credential_id {
                    storage.credentials().get_by_id(c_id).await.ok().flatten()
                } else {
                    None
                };

                println!("正在通过纯 Rust 协议引擎连接主机 {} ({}:{})...", host.name, host.address, host.port);
                let session_id = ssh_svc
                    .connect(&host, cred.as_ref())
                    .await
                    .map_err(|e| anyhow::anyhow!("建立纯 Rust SSH 会话失败: {e}"))?;

                println!("正在远程执行命令: '{}'", cmd);
                let output = ssh_svc
                    .execute_command(&session_id, &cmd)
                    .await
                    .map_err(|e| anyhow::anyhow!("远程执行命令失败: {e}"))?;

                if !output.stdout.is_empty() {
                    print!("{}", String::from_utf8_lossy(&output.stdout));
                }
                if !output.stderr.is_empty() {
                    eprint!("{}", String::from_utf8_lossy(&output.stderr));
                }

                let _ = ssh_svc.disconnect(&session_id).await;
                if output.exit_code != 0 {
                    std::process::exit(output.exit_code);
                }
            }
        },
        Commands::Keygen { algo, passphrase, bits, output } => {
            let key_algo = match algo.to_lowercase().as_str() {
                "rsa" => KeyAlgorithm::Rsa,
                "ecdsa" | "p256" => KeyAlgorithm::EcdsaP256,
                _ => KeyAlgorithm::Ed25519,
            };

            println!("正在基于纯 Rust 原生引擎生成 {:?} 密钥对...", key_algo);
            let pair = keygen_svc
                .generate_keypair(key_algo, bits, passphrase.as_deref())
                .map_err(|e| anyhow::anyhow!("密钥对生成失败: {e}"))?;

            println!("公钥指纹:   {}", pair.fingerprint);
            println!("公钥内容:   {}", pair.public_key_openssh);

            if let Some(path) = output {
                std::fs::write(&path, &pair.private_key_pem)
                    .with_context(|| format!("写入私钥文件失败: {path}"))?;
                let pub_path = format!("{}.pub", path);
                std::fs::write(&pub_path, &pair.public_key_openssh)
                    .with_context(|| format!("写入公钥文件失败: {pub_path}"))?;
                println!("已将私钥写入: {}", path);
                println!("已将公钥写入: {}", pub_path);
            } else {
                println!("\n----- 私钥内容 (请妥善保存) -----\n{}", pair.private_key_pem);
            }
        }
        Commands::Tunnel { sub } => match sub {
            TunnelCommands::List => {
                auth::ensure_vault_unlocked(&storage, cli.master_password.as_deref()).await?;
                let tunnels = storage.tunnels().list_all().await.context("读取隧道列表失败")?;
                println!("{:<16} {:<24} {:<10} {:<20} {:<20}", "ID", "NAME", "TYPE", "LOCAL", "REMOTE");
                println!("{}", "-".repeat(92));
                for t in tunnels {
                    let local = format!("{}:{}", t.local_bind, t.local_port);
                    let remote = format!("{}:{}", t.remote_host, t.remote_port);
                    println!(
                        "{:<16} {:<24} {:<10} {:<20} {:<20}",
                        t.id, t.name, t.tunnel_type.as_str(), local, remote
                    );
                }
            }
        },
        Commands::Version => {
            println!("Smalux CLI & TUI v{}", env!("CARGO_PKG_VERSION"));
            println!("模式: 双模驱动 (Headless CLI 命令行脚本化 + TUI 全屏交互仪表盘)");
            println!("架构: 纯 Rust 六边形架构 (Ports & Adapters)");
            println!("驱动引擎: NativeKeygenService, RusshSessionDriver, RusshTunnelDriver");
            println!("外部工具依赖: 0 (无需 ssh.exe, ssh-keygen, connect.exe, nc)");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cli_storage_initialization() {
        let storage = init_storage().await;
        let hosts = storage.hosts().list_all().await.unwrap();
        assert!(!hosts.is_empty(), "默认回退存储应包含预设的主机记录");
    }

    #[tokio::test]
    async fn test_cli_keygen_command() {
        let keygen_svc = NativeKeygenService::new();
        let pair = keygen_svc.generate_keypair(KeyAlgorithm::Ed25519, None, None).unwrap();
        assert!(pair.public_key_openssh.starts_with("ssh-ed25519 "));
        assert!(pair.private_key_pem.contains("OPENSSH PRIVATE KEY"));
    }
}
