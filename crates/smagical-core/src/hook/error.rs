//! 终端全场景强类型异常定义。

/// 强类型终端与网络异常枚举。
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum TerminalError {
    /// PTY 本地终端实例启动失败
    #[error("[{host_name} ({address}:{port})] PTY 实例启动失败: {reason}")]
    PtySpawnFailed {
        /// 目标主机显示名称
        host_name: String,
        /// 连接地址
        address: String,
        /// 端口
        port: u16,
        /// 失败详细原因
        reason: String,
    },

    /// 连接超时
    #[error("[{host_name} ({address}:{port})] 连接超时 ({timeout_ms}ms)")]
    ConnectTimeout {
        /// 目标主机显示名称
        host_name: String,
        /// 连接地址
        address: String,
        /// 端口
        port: u16,
        /// 超时毫秒数
        timeout_ms: u64,
    },

    /// 远端主机主动拒绝连接 (Connection Refused)
    #[error("[{host_name} ({address}:{port})] 目标主机拒绝连接 (Connection Refused)")]
    ConnectionRefused {
        /// 目标主机显示名称
        host_name: String,
        /// 连接地址
        address: String,
        /// 端口
        port: u16,
    },

    /// SSH 身份认证失败
    #[error("[{host_name} ({address}:{port})] SSH 认证失败 (用户: {username}, 方式: {method})")]
    AuthFailed {
        /// 目标主机显示名称
        host_name: String,
        /// 连接地址
        address: String,
        /// 端口
        port: u16,
        /// 登录用户名
        username: String,
        /// 认证方式
        method: String,
    },

    /// SSH 主机公钥指纹不匹配 (可能存在中间人攻击)
    #[error("[{host_name} ({address}:{port})] 主机公钥指纹变更 (预期: {expected}, 实际: {actual})")]
    HostKeyMismatch {
        /// 目标主机显示名称
        host_name: String,
        /// 连接地址
        address: String,
        /// 端口
        port: u16,
        /// 预期的公钥指纹
        expected: String,
        /// 实际接收到的公钥指纹
        actual: String,
    },

    /// 网络意外闪断或连接重置
    #[error("[{host_name} ({address}:{port})] 网络连接异常中断: {reason}")]
    ConnectionBroken {
        /// 目标主机显示名称
        host_name: String,
        /// 连接地址
        address: String,
        /// 端口
        port: u16,
        /// 中断原因
        reason: String,
    },

    /// 心跳探活超时
    #[error("[{host_name} ({address}:{port})] KeepAlive 心跳连续 {failed_probes} 次超时")]
    KeepAliveTimeout {
        /// 目标主机显示名称
        host_name: String,
        /// 连接地址
        address: String,
        /// 端口
        port: u16,
        /// 失败次数
        failed_probes: u32,
    },

    /// 命令执行被安全拦截
    #[error("[{host_name}] 命令 [{command}] 被安全策略拦截: {reason}")]
    CommandBlocked {
        /// 目标主机显示名称
        host_name: String,
        /// 拦截的命令
        command: String,
        /// 拦截原因
        reason: String,
    },

    /// 插件自身抛出异常或 Panic
    #[error("[{host_name}] 插件 [{plugin_name}] 执行异常: {reason}")]
    PluginFault {
        /// 目标主机显示名称
        host_name: String,
        /// 插件名称
        plugin_name: String,
        /// 错误原因
        reason: String,
    },

    /// 其他未分类未知错误
    #[error("[{host_name}] 未知终端错误: {message}")]
    Unknown {
        /// 目标主机显示名称
        host_name: String,
        /// 错误信息
        message: String,
    },
}
