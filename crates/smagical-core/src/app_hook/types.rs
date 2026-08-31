//! 全局应用级 Hook 数据模型与上下文定义。

use std::time::SystemTime;

/// 窗口全局状态枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WindowState {
    /// 正常前台展示。
    Normal,
    /// 最小化到任务栏。
    Minimized,
    /// 最大化全屏展示。
    Maximized,
    /// 获得焦点处于活动状态。
    Focused,
    /// 失去焦点处于后台状态 (可用于节能降频)。
    Unfocused,
}

/// 全局参数与配置变动事件上下文。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ConfigChangeEvent {
    /// 变动的配置项键名 (如 "terminal.font_size", "appearance.theme", "ssh.keepalive")。
    pub key: String,
    /// 变动前原值 (字符串序列化形式)。
    pub old_val: String,
    /// 变动后新值 (字符串序列化形式)。
    pub new_val: String,
    /// 变动来源 (如 "user_ui", "command_palette", "preset_import", "sync_engine")。
    pub source: String,
    /// 变动发生时的 UNIX 时间戳 (秒)。
    pub timestamp: u64,
}

impl ConfigChangeEvent {
    /// 创建一个新的配置变更事件实例。
    pub fn new(
        key: impl Into<String>,
        old_val: impl Into<String>,
        new_val: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            key: key.into(),
            old_val: old_val.into(),
            new_val: new_val.into(),
            source: source.into(),
            timestamp,
        }
    }

    /// 判断当前配置变更是否属于或影响指定的配置节/前缀 (例如: `event.affects("terminal")` 或 `event.affects("appearance.theme")`)。
    pub fn affects(&self, section_or_key: &str) -> bool {
        self.key == section_or_key || self.key.starts_with(&format!("{}.", section_or_key))
    }

    /// 判断参数值是否实际发生了变动 (即新值不等于旧值)。
    pub fn is_changed(&self) -> bool {
        self.old_val != self.new_val
    }
}


/// 应用程序启动引导上下文。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AppBootContext {
    /// 启动时传入的命令行参数列表。
    pub cli_args: Vec<String>,
    /// 进程启动时间戳 (UNIX 秒)。
    pub started_at: u64,
}

impl AppBootContext {
    /// 创建默认的应用启动上下文。
    pub fn new(cli_args: Vec<String>) -> Self {
        let started_at = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            cli_args,
            started_at,
        }
    }
}

/// 应用程序退出与关闭上下文。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AppExitContext {
    /// 是否为强制退出 (如系统关机信号或未捕获致命错误)。
    pub is_forced: bool,
    /// 当前仍处于活跃状态的终端会话数。
    pub active_sessions_count: usize,
    /// 退出状态码 (0 表示正常退出)。
    pub exit_code: i32,
}

impl AppExitContext {
    /// 构造标准正常退出上下文。
    pub fn normal(active_sessions_count: usize) -> Self {
        Self {
            is_forced: false,
            active_sessions_count,
            exit_code: 0,
        }
    }

    /// 构造强制退出上下文。
    pub fn forced(exit_code: i32) -> Self {
        Self {
            is_forced: true,
            active_sessions_count: 0,
            exit_code,
        }
    }
}
