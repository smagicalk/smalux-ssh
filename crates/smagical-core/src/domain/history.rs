//! 历史会话领域模型 (History Record)
//!
//! 记录每次终端连接的目标、时间、持续时长与退出健康状态，支持一键重连、分屏打开与置顶收藏。

use serde::{Deserialize, Serialize};

/// 终端连接历史记录实体
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryRecord {
    /// 历史记录唯一 UUID (如 "hist-1700000000000")
    pub id: String,
    /// 关联的主机资产 ID (如果直接从主机库启动；本地终端为 None)
    pub host_id: Option<String>,
    /// 会话显示名称 (如 "prod-api-server", "PowerShell #1")
    pub title: String,
    /// 目标网络地址与端口 (如 "192.168.1.100:22" 或 "Local Shell")
    pub address: String,
    /// 目标端口 (如 22)
    pub port: u16,
    /// 登录用户名 (如 "root", "admin")
    pub username: String,
    /// 协议或终端类别 ("ssh" | "local" | "sftp")
    pub session_type: String,
    /// 连接建立时间戳 (Unix 秒)
    pub connected_at: u64,
    /// 连接断开时间戳 (Unix 秒，活跃中为 None)
    pub disconnected_at: Option<u64>,
    /// 持续总秒数 (如 2520 -> 42m)
    pub duration_secs: u64,
    /// 退出健康状态 ("active" | "success" | "timeout" | "auth_failed" | "error")
    pub exit_status: String,
    /// 错误诊断简讯 (如 "Connection refused on port 22")
    pub error_msg: Option<String>,
    /// 是否被用户置顶标星 (置顶项永不被自动淘汰)
    pub is_pinned: bool,
    /// 累计连接频次 (用于在按主机聚合视图下排序)
    pub connect_count: u32,
    /// 是否已保存该会话的终端屏幕输出快照
    #[serde(default)]
    pub has_snapshot: bool,
    /// 会话快照保存的实际行数
    #[serde(default)]
    pub snapshot_lines: u32,
}

/// 终端会话屏幕快照策略配置 (支持在系统设置中动态自定义)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSnapshotConfig {
    /// 是否在会话关闭时自动捕获并持久化终端屏幕快照 (默认为 true)
    pub enable_auto_snapshot: bool,
    /// 单个会话快照最大保留行数 (默认 500 行，0 为全量保留)
    pub max_snapshot_lines: usize,
    /// 快照在本地存储中的最长保留天数 (默认 30 天，0 为永久保存)
    pub retention_days: u32,
}

impl Default for SessionSnapshotConfig {
    fn default() -> Self {
        Self {
            enable_auto_snapshot: true,
            max_snapshot_lines: 500,
            retention_days: 30,
        }
    }
}

impl HistoryRecord {

    /// 创建新的 SSH 历史会话记录 (初始为活跃中状态)
    pub fn new_ssh(
        id: String,
        host_id: Option<String>,
        title: String,
        address: String,
        port: u16,
        username: String,
        connected_at: u64,
    ) -> Self {
        Self {
            id,
            host_id,
            title,
            address,
            port,
            username,
            session_type: "ssh".to_string(),
            connected_at,
            disconnected_at: None,
            duration_secs: 0,
            exit_status: "active".to_string(),
            error_msg: None,
            is_pinned: false,
            connect_count: 1,
            has_snapshot: false,
            snapshot_lines: 0,
        }
    }

    /// 创建新的本地 Shell 历史记录
    pub fn new_local(
        id: String,
        host_id: Option<String>,
        shell_type: String,
        title: String,
        connected_at: u64,
    ) -> Self {
        Self {
            id,
            host_id,
            title,
            address: format!("Local ({})", shell_type),
            port: 0,
            username: "local".to_string(),
            session_type: "local".to_string(),
            connected_at,
            disconnected_at: None,
            duration_secs: 0,
            exit_status: "active".to_string(),
            error_msg: None,
            is_pinned: false,
            connect_count: 1,
            has_snapshot: false,
            snapshot_lines: 0,
        }
    }


    /// 标记连接正常结束
    pub fn mark_closed(&mut self, disconnected_at: u64) {
        self.disconnected_at = Some(disconnected_at);
        if disconnected_at >= self.connected_at {
            self.duration_secs = disconnected_at - self.connected_at;
        }
        if self.exit_status == "active" {
            self.exit_status = "success".to_string();
        }
    }

    /// 标记关联了终端屏幕快照
    pub fn record_snapshot(&mut self, lines_count: u32) {
        self.has_snapshot = true;
        self.snapshot_lines = lines_count;
    }

    /// 标记连接异常终止
    pub fn mark_failed(&mut self, disconnected_at: u64, reason: &str, error_msg: Option<String>) {
        self.disconnected_at = Some(disconnected_at);
        if disconnected_at >= self.connected_at {
            self.duration_secs = disconnected_at - self.connected_at;
        }
        self.exit_status = reason.to_string();
        self.error_msg = error_msg;
    }
}

