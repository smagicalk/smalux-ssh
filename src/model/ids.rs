//! 稳定标识和敏感引用。

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 主机唯一标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HostId(pub Uuid);

/// 主机分组唯一标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GroupId(pub Uuid);

/// 连接会话唯一标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub Uuid);

/// 默认本地终端会话 ID。
pub const LOCAL_TERMINAL_SESSION_ID: SessionId =
    SessionId(Uuid::from_u128(0x11111111111111111111111111111111));

/// 默认本地终端标题。
#[cfg(windows)]
pub const DEFAULT_LOCAL_TERMINAL_TITLE: &str = "Local Shell";

/// 默认本地终端标题。
#[cfg(not(windows))]
pub const DEFAULT_LOCAL_TERMINAL_TITLE: &str = "Local Shell";

/// 命令历史唯一标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommandHistoryId(pub Uuid);

/// SFTP 传输任务唯一标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransferId(pub Uuid);

/// 工作区唯一标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(pub Uuid);

/// 快捷命令唯一标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SnippetId(pub Uuid);

/// 敏感凭据的间接引用。
///
/// 实际密码、私钥口令和证书材料应由系统凭据库或加密存储保存。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SecretRef(pub String);
