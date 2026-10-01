//! 统一网络与协议核心服务契约层 (Core Service Contracts)。
//!
//! 基于六边形架构 (Ports & Adapters) 与依赖倒置原则 (DIP) 设计。
//! 在此定义系统所有远程协议操作的 Trait 契约，使上层 UI 与 CLI 彻底脱离具体网络实现与宿主外部工具依赖。

/// 统一错误模型与结果类型。
pub mod error;
/// SSH 密钥对生成与格式转换服务契约。
pub mod keygen;
/// 远程主机系统性能指标采集服务契约。
pub mod metrics;
/// 纯内存网络协议仿真实现 (用于单测与无网络调试)。
pub mod mock;
/// SFTP 远程文件传输与目录管理服务契约。
pub mod sftp;
/// SSH 远程会话连接与虚拟终端通道契约。
pub mod ssh;
/// 网络隧道与端口转发服务契约。
pub mod tunnel;
/// 容灾备份与多端快照同步服务契约。
pub mod backup;

pub use error::{SshServiceError, SshServiceResult};
pub use keygen::{GeneratedKeyPair, KeyAlgorithm, KeygenService};
pub use metrics::{HostMetricsService, SystemMetricsSnapshot};
pub use mock::{
    MockHostMetricsService, MockKeygenService, MockSftpService, MockSshSessionService,
    MockTunnelService,
};
pub use sftp::{SftpService, TransferProgress};
pub use ssh::{CommandExecutionOutput, HostKeyVerificationResult, SshProgressCallback, SshSessionService, SshStreamChannel};
pub use tunnel::{TunnelHandle, TunnelMetricsSnapshot, TunnelService};
pub use backup::{
    compute_sha256, create_backup_driver, create_backup_payload, format_bytes_size,
    restore_backup_payload, BackupDriver, GistBackupDriver, LocalBackupDriver,
    RemoteSnapshotInfo, S3BackupDriver, WebdavBackupDriver,
};
