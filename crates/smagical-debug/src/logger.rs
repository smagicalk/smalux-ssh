//! 开发者实时事件日志管理服务 (Log Buffer Manager)

use std::collections::VecDeque;
use crate::models::DebugLogEntry;

/// 获取当前时间字符串 (格式: "HH:mm:ss", UTC+8 偏移)
pub fn get_current_timestamp() -> String {
    let now = std::time::SystemTime::now();
    let since_epoch = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let total_secs = since_epoch.as_secs();
    let local_secs = (total_secs + 28800) % 86400;
    let hours = local_secs / 3600;
    let mins = (local_secs % 3600) / 60;
    let secs = local_secs % 60;
    format!("{:02}:{:02}:{:02}", hours, mins, secs)
}

/// 开发者调试日志循环缓冲区
#[derive(Clone, Debug)]
pub struct DebugLogBuffer {
    entries: VecDeque<DebugLogEntry>,
    max_capacity: usize,
}

impl Default for DebugLogBuffer {
    fn default() -> Self {
        Self::new(200)
    }
}

impl DebugLogBuffer {
    /// 创建指定最大容量的日志缓冲区 (默认 200 条)
    pub fn new(max_capacity: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_capacity),
            max_capacity,
        }
    }

    /// 压入一条新日志 (最新日志插入最前)
    pub fn push(&mut self, level: &str, module: &str, message: &str) -> DebugLogEntry {
        let entry = DebugLogEntry::new(get_current_timestamp(), level, module, message);
        self.push_entry(entry.clone());
        entry
    }

    /// 直接压入一个已有日志条目对象
    pub fn push_entry(&mut self, entry: DebugLogEntry) {
        self.entries.push_front(entry);
        if self.entries.len() > self.max_capacity {
            self.entries.pop_back();
        }
    }

    /// 获取所有日志条目 (按时间从新到旧排列)
    pub fn get_all(&self) -> Vec<DebugLogEntry> {
        self.entries.iter().cloned().collect()
    }

    /// 清空所有日志
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// 当前日志总数量
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 日志是否为空
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
