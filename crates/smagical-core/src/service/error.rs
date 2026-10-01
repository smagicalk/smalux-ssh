//! 核心协议与网络服务统一错误模型。

use thiserror::Error;

/// 协议与远程服务统一错误枚举
#[derive(Debug, Error)]
pub enum SshServiceError {
    /// 身份认证失败（密码错误、私钥不受信任、Passphrase 错误等）
    #[error("身份认证失败: {0}")]
    AuthFailed(String),

    /// 目标主机不可达或拒绝连接
    #[error("远程主机不可达: {0}")]
    HostUnreachable(String),

    /// 网络操作超时
    #[error("网络超时: {0}")]
    Timeout(String),

    /// 权限拒绝（文件无读写权限、远程命令受限等）
    #[error("权限拒绝: {0}")]
    PermissionDenied(String),

    /// 路径未找到（远程文件或目录不存在）
    #[error("路径未找到: {0}")]
    NotFound(String),

    /// 协议交互异常或协商失败
    #[error("协议异常: {0}")]
    ProtocolError(String),

    /// 会话或通道已关闭
    #[error("通道已断开: {0}")]
    ChannelClosed(String),

    /// 磁盘空间不足或配额受限
    #[error("磁盘空间不足: {0}")]
    DiskFull(String),

    /// 本地或远程 IO 读写异常
    #[error("底层 IO 异常: {0}")]
    Io(#[from] std::io::Error),

    /// 依赖服务未就绪或内部实现异常
    #[error("内部服务异常: {0}")]
    Internal(String),

    /// 安全防线警报 (如主机公钥指纹不匹配，疑似遭遇中间人劫持攻击)
    #[error("安全风险警报: {0}")]
    SecurityAlert(String),
}

/// 服务操作统一结果类型
pub type SshServiceResult<T> = Result<T, SshServiceError>;
