//! 代码片段智能参数记忆与高频调用追踪服务 (Snippet Intelligence Service)。
//!
//! # 架构职责
//! 1. **参数动态记忆**：跨会话、跨生命周期持久化占位符历史输入值；
//! 2. **调用频次追踪与推荐**：自动记录代码片段执行次数与最近调用时间，通过评分算法智能置顶高频运维命令；
//! 3. **物理音效反馈**：提供一键注入终端时的微秒级触觉/按键回响 (`play_snippet_injection_feedback`)。

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

use smagical_core::domain::snippet::{SnippetParamMemory, SnippetUsageTracker};

static PARAM_MEMORY: OnceLock<RwLock<SnippetParamMemory>> = OnceLock::new();
static USAGE_TRACKER: OnceLock<RwLock<SnippetUsageTracker>> = OnceLock::new();

/// 获取全局单例代码片段动态参数记忆
pub fn get_param_memory() -> &'static RwLock<SnippetParamMemory> {
    PARAM_MEMORY.get_or_init(|| RwLock::new(load_snippet_param_memory()))
}

/// 获取全局单例代码片段使用度量追踪器
pub fn get_usage_tracker() -> &'static RwLock<SnippetUsageTracker> {
    USAGE_TRACKER.get_or_init(|| RwLock::new(load_snippet_usage_tracker()))
}

/// 解析并获取持久化代码片段智能记忆与度量数据的存储根目录。
///
/// 遵循操作系统的通用应用数据目录规范：
/// - Windows: `%LOCALAPPDATA%\smagical\smalux\snippets`
/// - Linux: `~/.local/share/smalux/snippets`
/// - macOS: `~/Library/Application Support/com.smagical.smalux/snippets`
pub fn get_snippet_intelligence_dir() -> PathBuf {
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "smagical", "smalux") {
        let dir = proj_dirs.data_local_dir().join("snippets");
        if fs::create_dir_all(&dir).is_ok() {
            return dir;
        }
    }

    if let Some(user_dirs) = directories::UserDirs::new() {
        let dir = user_dirs.home_dir().join(".smalux").join("snippets");
        if fs::create_dir_all(&dir).is_ok() {
            return dir;
        }
    }

    let fallback = PathBuf::from("snippets");
    let _ = fs::create_dir_all(&fallback);
    fallback
}

/// 从物理磁盘加载代码片段动态参数记忆
pub fn load_snippet_param_memory() -> SnippetParamMemory {
    let file_path = get_snippet_intelligence_dir().join("param_memory.json");
    if file_path.exists() {
        if let Ok(content) = fs::read_to_string(&file_path) {
            if let Ok(mem) = SnippetParamMemory::from_json(&content) {
                tracing::info!(
                    target: "smagical_ui::snippets",
                    "已从磁盘恢复代码片段参数记忆 (片段数: {}, 全局参数: {})",
                    mem.scoped_params.len(),
                    mem.global_params.len()
                );
                return mem;
            }
        }
    }
    SnippetParamMemory::new()
}

/// 异步非阻塞持久化代码片段动态参数记忆
pub fn save_snippet_param_memory_async(memory: SnippetParamMemory) {
    tokio::task::spawn_blocking(move || {
        let dir = get_snippet_intelligence_dir();
        let file_path = dir.join("param_memory.json");
        if let Ok(json_str) = memory.to_json() {
            if let Err(e) = fs::write(&file_path, json_str) {
                tracing::warn!(target: "smagical_ui::snippets", "写入代码片段参数记忆失败: {}", e);
            }
        }
    });
}

/// 从物理磁盘加载代码片段使用度量追踪数据
pub fn load_snippet_usage_tracker() -> SnippetUsageTracker {
    let file_path = get_snippet_intelligence_dir().join("usage_tracker.json");
    if file_path.exists() {
        if let Ok(content) = fs::read_to_string(&file_path) {
            if let Ok(tracker) = SnippetUsageTracker::from_json(&content) {
                tracing::info!(
                    target: "smagical_ui::snippets",
                    "已从磁盘恢复代码片段使用度量 (已追踪指令数: {})",
                    tracker.usage.len()
                );
                return tracker;
            }
        }
    }
    SnippetUsageTracker::new()
}

/// 异步非阻塞持久化代码片段使用度量追踪数据
pub fn save_snippet_usage_tracker_async(tracker: SnippetUsageTracker) {
    tokio::task::spawn_blocking(move || {
        let dir = get_snippet_intelligence_dir();
        let file_path = dir.join("usage_tracker.json");
        if let Ok(json_str) = tracker.to_json() {
            if let Err(e) = fs::write(&file_path, json_str) {
                tracing::warn!(target: "smagical_ui::snippets", "写入代码片段使用度量失败: {}", e);
            }
        }
    });
}

/// 触发代码片段注入终端的按键/音效物理反馈 (0ms 零阻塞低延迟，标准终端蜂鸣)
pub fn play_snippet_injection_feedback() {
    let _ = std::io::stdout().write_all(b"\x07");
    let _ = std::io::stdout().flush();
}

/// 统一记录代码片段执行事件：更新使用频次、持久化填报参数并触发物理音效反馈
///
/// # 参数
/// - `snippet_id`: 被执行的代码片段 ID；
/// - `session_id`: 目标终端会话 ID (可选)；
/// - `params`: 本次注入时用户填写的占位符键值对映射 (可选)。
pub fn record_snippet_execution(
    snippet_id: &str,
    session_id: Option<&str>,
    params: Option<&HashMap<String, String>>,
) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // 1. 更新频次与时间戳追踪
    let tracker_clone = {
        let mut tracker = get_usage_tracker().write().unwrap();
        tracker.record_execution(snippet_id, session_id, now);
        tracker.clone()
    };
    save_snippet_usage_tracker_async(tracker_clone);

    // 2. 更新参数动态记忆 (若有)
    if let Some(params_map) = params {
        let mem_clone = {
            let mut memory = get_param_memory().write().unwrap();
            memory.remember_all(snippet_id, params_map);
            memory.clone()
        };
        save_snippet_param_memory_async(mem_clone);
    }

    // 3. 触发操作音频/按键反馈
    play_snippet_injection_feedback();
}
