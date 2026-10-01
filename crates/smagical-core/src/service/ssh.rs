//! SSH 远程会话连接与交互式终端通道契约。

use crate::domain::{CredentialRecord, HostRecord};
use crate::service::error::SshServiceResult;
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};

/// 抽象的终端连续双向字节读写流通道 (与上层 UI 软光栅渲染器解耦)
pub trait SshStreamChannel: AsyncRead + AsyncWrite + Send + Sync + Unpin {}

/// 为任何满足对应约束的类型自动实现 `SshStreamChannel`
impl<T> SshStreamChannel for T where T: AsyncRead + AsyncWrite + Send + Sync + Unpin {}

/// 单次非交互式远程命令执行返回结果
#[derive(Debug, Clone)]
pub struct CommandExecutionOutput {
    /// 命令退出码 (0 表示成功)
    pub exit_code: i32,
    /// 标准输出字节内容
    pub stdout: Vec<u8>,
    /// 标准错误字节内容
    pub stderr: Vec<u8>,
}

/// 远程主机公钥验真结果 (用于 TOFU 机制与中间人防劫持)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostKeyVerificationResult {
    /// 公钥完全受信任 (已存在于 known_hosts 且与历史记录一致)
    Trusted,
    /// 首次连接该主机 (未记录过历史公钥，附带公钥指纹与 OpenSSH 格式文本)
    FirstTimeHost {
        /// 主机公钥指纹 (例如 SHA256:xxx)
        fingerprint: String,
        /// 公钥 OpenSSH 格式文本
        public_key_text: String,
    },
    /// 严重安全警报：远端主机公钥与已知历史记录不匹配 (疑似遭遇中间人攻击劫持！)
    Mismatch {
        /// 历史已知公钥指纹
        expected_fingerprint: String,
        /// 实际接收到的未知公钥指纹
        actual_fingerprint: String,
    },
}

use std::sync::Arc;

/// SSH 连接进度阶段回调类型 (接收人类可读的当前阶段状态描述字符串)
pub type SshProgressCallback = Arc<dyn Fn(&str) + Send + Sync>;

/// SSH 远程会话管理契约
#[async_trait]
pub trait SshSessionService: Send + Sync {
    /// 建立远程 SSH 会话连接 (返回会话唯一 session_id)
    async fn connect(
        &self,
        host: &HostRecord,
        credential: Option<&CredentialRecord>,
    ) -> SshServiceResult<String> {
        self.connect_with_progress(host, credential, None).await
    }

    /// 建立远程 SSH 会话连接并实时汇报连接阶段进度
    async fn connect_with_progress(
        &self,
        host: &HostRecord,
        credential: Option<&CredentialRecord>,
        progress: Option<SshProgressCallback>,
    ) -> SshServiceResult<String>;

    /// 打开交互式 PTY 虚拟终端通道
    async fn open_pty_channel(
        &self,
        session_id: &str,
        term_type: &str,
        rows: u16,
        cols: u16,
    ) -> SshServiceResult<Box<dyn SshStreamChannel>>;

    /// 动态调整 PTY 视口窗口尺寸
    async fn resize_pty(&self, session_id: &str, rows: u16, cols: u16) -> SshServiceResult<()>;

    /// 单次非交互式执行远程命令
    async fn execute_command(&self, session_id: &str, command: &str) -> SshServiceResult<CommandExecutionOutput>;

    /// 断开并释放指定 SSH 会话
    async fn disconnect(&self, session_id: &str) -> SshServiceResult<()>;
}
