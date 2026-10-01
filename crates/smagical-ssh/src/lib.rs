//! # Smagical SSH - 独立无 UI 核心协议与远程运维引擎
//!
//! 提供系统级原生的 SSH/SFTP 进程管道、私钥生命周期防护、密钥对生成、外部资产导入及 Linux 性能指标采样：
//! - `ssh_config`: 会话参数构造器与 RAII `KeyTempGuard` 物理抹零私钥守卫；
//! - `keygen`: 现场 Ed25519/RSA/ECDSA 密钥对生成；
//! - `importer`: Termius, Xshell, CSV/TSV 资产导入无损解析；
//! - `sftp`: 远程目录列表、文件双向流式传输、目录创建与删除；
//! - `monitor`: Linux 内核级指标原子采样与 30s 平滑 SVG 时序波形计算。

#![deny(missing_docs)]

/// SSH 启动参数与 RAII 临时私钥守卫。
pub mod ssh_config;
/// SSH 密钥对现场生成器。
pub mod keygen;
/// 第三方 SSH 资产配置导入解析器。
pub mod importer;
/// 远程 SFTP 与文件系统协议操作引擎。
pub mod sftp;
/// 纯 Rust 原生 SFTP 驱动与服务实现。
pub mod sftp_driver;
/// 纯 Rust 原生 SSH 远程终端与会话服务驱动实现。
pub mod session_driver;
/// 纯 Rust 原生网络隧道与端口转发驱动实现。
pub mod tunnel_driver;
/// 纯 Rust 原生系统性能监控指标采集驱动实现。
pub mod metrics_driver;
/// 真实 Linux 系统运维性能指标探针引擎。
pub mod monitor;
/// OpenSSH Known Hosts 原生文件管理与主机公钥验真引擎 (TOFU)。
pub mod known_hosts;

pub use ssh_config::{execute_remote, KeyTempGuard, SshLaunchConfig};
pub use keygen::{generate_ssh_keypair, NativeKeygenService};
pub use sftp_driver::RusshSftpDriver;
pub use session_driver::RusshSessionDriver;
pub use tunnel_driver::RusshTunnelDriver;
pub use metrics_driver::RusshMetricsDriver;
pub use importer::{ImportedHostEntry, get_default_ssh_config_path, parse_external_assets, parse_ssh_config};
pub use sftp::{
    AskPassGuard, SftpCommandContext,
    list_remote_directory, upload_path, download_path,
    create_remote_dir, create_remote_file, remove_remote_path, rename_remote_path,
    parse_ls_output,
};
pub use monitor::{
    CpuJiffies, NetSnapshot, SparklineHistory, LinuxSystemMetrics, LinuxMetricsSampler,
    parse_linux_metrics_output, format_bytes, format_rate,
};
pub use known_hosts::{
    append_known_host_to_file, get_default_known_hosts_path, verify_server_key_in_file,
    KnownHostEntry,
};
