//! 容灾备份与快照管理领域模型 (Backup & Disaster Recovery Domain Model)

use serde::{Deserialize, Serialize};

/// 备份存储协议类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupType {
    /// 本地文件系统目录
    Local,
    /// WebDAV 协议 (坚果云 / NAS / Nextcloud)
    Webdav,
    /// Amazon S3 兼容对象存储 (AWS S3, 阿里云 OSS, MinIO, R2)
    S3,
    /// GitHub Gist
    Gist,
    /// 自建服务
    SelfHosted,
}

impl BackupType {
    /// 转换为静态协议标识字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Webdav => "webdav",
            Self::S3 => "s3",
            Self::Gist => "gist",
            Self::SelfHosted => "self_hosted",
        }
    }

    /// 从字符串解析为协议类型
    pub fn from_str(s: &str) -> Self {
        match s {
            "webdav" => Self::Webdav,
            "s3" => Self::S3,
            "gist" => Self::Gist,
            "self_hosted" => Self::SelfHosted,
            _ => Self::Local,
        }
    }
}

/// 自动备份同步触发策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackupStrategy {
    /// 实时同步 (监听资产与配置变动事件，防抖触发)
    Realtime,
    /// 每天定时
    Daily,
    /// 每小时
    Hourly,
    /// 软件安全退出时
    OnExit,
    /// 仅手动同步
    Manual,
}

impl BackupStrategy {
    /// 转换为静态策略标识字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Realtime => "realtime",
            Self::Daily => "daily",
            Self::Hourly => "hourly",
            Self::OnExit => "on_exit",
            Self::Manual => "manual",
        }
    }

    /// 从字符串解析为调度策略
    pub fn from_str(s: &str) -> Self {
        match s {
            "realtime" => Self::Realtime,
            "hourly" => Self::Hourly,
            "on_exit" => Self::OnExit,
            "manual" => Self::Manual,
            _ => Self::Daily,
        }
    }
}

/// 容灾备份任务配置持久化记录
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupTaskRecord {
    /// 任务唯一 ID
    pub id: String,
    /// 任务名称
    pub name: String,
    /// 存储类型
    pub backup_type: BackupType,
    /// 目标端点 / 路径 / Bucket / Gist ID
    pub endpoint: String,
    /// 认证用户 / Access Key / GitHub PAT
    pub auth_user: String,
    /// 认证密钥 / Secret Key
    pub auth_secret: String,
    /// 触发调度策略
    pub strategy: BackupStrategy,
    /// 保留规则摘要 (如 "保留最近 10 份")
    pub retention: String,
    /// 是否已启用该节点
    pub enabled: bool,
    /// 最近一次备份时间戳 (格式化字符串或空)
    pub last_backup_time: String,
    /// 历史快照版本累计数量
    pub snapshot_count: u32,
    /// 最近一次执行状态 ("idle", "success", "failed")
    pub last_status: String,
    /// 最近一次错误信息
    pub last_error: String,
    /// 创建时间 (UNIX 秒)
    pub created_at: u64,
    /// 更新时间 (UNIX 秒)
    pub updated_at: u64,
}

/// 备份快照版本镜像元数据记录
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupSnapshotRecord {
    /// 快照 ID (如 "snap-001")
    pub id: String,
    /// 所属任务 ID
    pub task_id: String,
    /// 格式化时间戳 (如 "2026-09-24 21:00:15")
    pub timestamp: String,
    /// 时间戳秒数
    pub epoch_secs: u64,
    /// 格式化大小 (如 "1.4 MB")
    pub size_str: String,
    /// 字节大小
    pub size_bytes: u64,
    /// 快照备注
    pub remark: String,
    /// SHA-256 完整性哈希 (如 "sha256:...")
    pub hash: String,
    /// 远端存储物理路径或资源 ID
    pub remote_id: String,
}
