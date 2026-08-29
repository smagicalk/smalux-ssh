//! 开发者调试模型定义 (Data Models)

use serde::{Deserialize, Serialize};

/// 开发者调试日志等级
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DebugLogLevel {
    /// 调试信息
    Debug,
    /// 普通运行时信息
    Info,
    /// 警告通知
    Warn,
    /// 严重错误
    Error,
}

impl DebugLogLevel {
    /// 转换为大写字符串切片
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }

    /// 从字符串解析日志等级 (不区分大小写)
    pub fn from_str_loose(s: &str) -> Self {
        match s.trim().to_uppercase().as_str() {
            "DEBUG" => Self::Debug,
            "WARN" | "WARNING" => Self::Warn,
            "ERROR" | "ERR" => Self::Error,
            _ => Self::Info,
        }
    }
}

/// 开发者实时事件日志条目 (Log Entry)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugLogEntry {
    /// 时间戳 (格式: "HH:mm:ss")
    pub timestamp: String,
    /// 日志等级 ("INFO", "DEBUG", "WARN", "ERROR")
    pub level: String,
    /// 来源模块 ("SYS", "UI", "THEME", "TREE", "DATA", "PRESET", "CMD", "SESSION", "SEARCH")
    pub module: String,
    /// 详细日志文本
    pub message: String,
}

impl DebugLogEntry {
    /// 创建新的日志条目
    pub fn new(
        timestamp: impl Into<String>,
        level: impl Into<String>,
        module: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            timestamp: timestamp.into(),
            level: level.into(),
            module: module.into(),
            message: message.into(),
        }
    }
}

/// 调试树形节点原始数据模型 (Raw Tree Node for debugging & state management)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugRawNode {
    /// 节点唯一 ID (如: "grp-prod", "host-k8s-01")
    pub id: String,
    /// 展示名称
    pub name: String,
    /// 是否为分组节点
    pub is_group: bool,
    /// 直接上级父节点 ID (根节点为空字符串 "")
    pub parent_id: String,
    /// 树状层级深度 (0 为根，1 为一级，2 为二级...)
    pub level: i32,
    /// 主机 IP 或域名 (仅主机有效)
    pub address: String,
    /// SSH 端口 (仅主机有效)
    pub port: i32,
    /// 在线健康状态 ("online", "warning", "offline")
    pub status: String,
    /// ICMP 延迟测速 (毫秒)
    pub ping_ms: i32,
    /// 分组下包含的主机/子节点总数量 (仅分组有效)
    pub item_count: i32,
}

/// 调试主机卡片数据模型 (Host Card Model)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugHostCard {
    /// 主机唯一 ID
    pub id: String,
    /// 主机别名
    pub name: String,
    /// IP 地址或域名
    pub address: String,
    /// SSH 端口
    pub port: i32,
    /// 所属分组展示名
    pub group: String,
    /// 在线健康状态 ("online", "warning", "offline")
    pub status: String,
    /// 延迟 (毫秒)
    pub ping_ms: i32,
}
