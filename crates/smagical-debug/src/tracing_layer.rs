//! 全局 tracing 日志子系统集成 (Console + Rolling File + UI Event Layer)

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime};

use tracing::field::{Field, Visit};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::layer::Context;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter, Layer};

use crate::logger::{get_current_timestamp, DebugLogBuffer};
use crate::models::DebugLogEntry;

use std::sync::atomic::{AtomicBool, Ordering};

/// 全局共享的 UI 实时日志缓冲队列
static GLOBAL_LOG_BUFFER: OnceLock<Arc<Mutex<DebugLogBuffer>>> = OnceLock::new();

/// 全局 Debug 调试功能开启/关闭开关 (默认开启 true)
static IS_DEBUG_ENABLED: AtomicBool = AtomicBool::new(true);

/// 全局 UI 日志捕获通道独立开关 (默认开启 true，与 debug 模式解绑)
static IS_LOG_CAPTURE_ENABLED: AtomicBool = AtomicBool::new(true);

/// 设置全局 Debug 功能开启/关闭状态。
pub fn set_debug_enabled(enabled: bool) {
    IS_DEBUG_ENABLED.store(enabled, Ordering::SeqCst);
}

/// 查询全局 Debug 功能是否处于开启状态。
pub fn is_debug_enabled() -> bool {
    IS_DEBUG_ENABLED.load(Ordering::SeqCst)
}

/// 设置全局 UI 日志捕获通道开启/关闭状态。
///
/// 若关闭捕获，将自动清空当前内存中的日志缓冲区，并停止捕获新的 UI 诊断日志。
pub fn set_log_capture_enabled(enabled: bool) {
    IS_LOG_CAPTURE_ENABLED.store(enabled, Ordering::SeqCst);
    if !enabled
        && let Ok(mut buf) = get_global_log_buffer().lock()
    {
        buf.clear();
    }
}

/// 查询全局 UI 日志捕获通道是否处于开启状态。
pub fn is_log_capture_enabled() -> bool {
    IS_LOG_CAPTURE_ENABLED.load(Ordering::SeqCst)
}

/// 获取或初始化全局 UI 日志缓冲区
pub fn get_global_log_buffer() -> Arc<Mutex<DebugLogBuffer>> {
    GLOBAL_LOG_BUFFER
        .get_or_init(|| Arc::new(Mutex::new(DebugLogBuffer::default())))
        .clone()
}


/// 字段值访问提取器
#[derive(Default)]
struct MessageVisitor {
    message: String,
    fields: Vec<(String, String)>,
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        let name = field.name();
        let val_str = format!("{:?}", value);
        if name == "message" {
            // 去除最外层可能被 debug 打印多包的引号
            let trimmed = val_str.trim_matches('"');
            self.message = trimmed.to_string();
        } else {
            self.fields.push((name.to_string(), val_str));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.fields.push((field.name().to_string(), value.to_string()));
        }
    }
}

/// 专为 smalux UI 调试面板捕获日志的 Subscriber Layer
#[derive(Clone)]
pub struct UiLogLayer {
    buffer: Arc<Mutex<DebugLogBuffer>>,
}

impl UiLogLayer {
    /// 创建新的 UI 日志捕获层
    pub fn new(buffer: Arc<Mutex<DebugLogBuffer>>) -> Self {
        Self { buffer }
    }
}

impl<S> Layer<S> for UiLogLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if !is_log_capture_enabled() {
            return;
        }

        let meta = event.metadata();

        
        // 过滤掉部分过于嘈杂的外部依赖内部 trace/debug（如 winit, slint 内部布局渲染等）
        let target = meta.target();
        if (target.starts_with("winit") || target.starts_with("calloop") || target.starts_with("slint::"))
            && *meta.level() > Level::INFO
        {
            return;
        }

        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let mut final_msg = visitor.message;
        if final_msg.is_empty() && !visitor.fields.is_empty() {
            let field_strs: Vec<String> = visitor
                .fields
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            final_msg = field_strs.join(", ");
        }

        let level_str = match *meta.level() {
            Level::ERROR => "ERROR",
            Level::WARN => "WARN",
            Level::INFO => "INFO",
            Level::DEBUG => "DEBUG",
            Level::TRACE => "TRACE",
        };

        // 规范化模块名称展示
        let module_name = if target.starts_with("smagical_ui") {
            "UI"
        } else if target.starts_with("smagical_core") {
            "CORE"
        } else if target.starts_with("smagical_debug") {
            "DEBUG"
        } else if target.contains("::") {
            target.split("::").last().unwrap_or(target)
        } else {
            target
        };

        if let Ok(mut buf) = self.buffer.lock() {
            let entry = DebugLogEntry::new(get_current_timestamp(), level_str, module_name, final_msg);
            buf.push_entry(entry);
        }
    }
}

/// Tracing 资源守护句柄 (保留非阻塞写入线程存活)
pub struct TracingGuard {
    _file_guard: tracing_appender::non_blocking::WorkerGuard,
}

/// 获取默认持久化日志存储目录
///
/// Windows: `%APPDATA%\smalux\logs` 或 `~/.smalux/logs`
pub fn get_default_log_dir(app_name: &str) -> PathBuf {
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "smagical", app_name) {
        let dir = proj_dirs.data_local_dir().join("logs");
        if fs::create_dir_all(&dir).is_ok() {
            return dir;
        }
    }

    if let Some(user_dirs) = directories::UserDirs::new() {
        let dir = user_dirs.home_dir().join(format!(".{}", app_name)).join("logs");
        if fs::create_dir_all(&dir).is_ok() {
            return dir;
        }
    }

    let fallback = PathBuf::from("logs");
    let _ = fs::create_dir_all(&fallback);
    fallback
}

/// 自动清理过期与超限的旧日志文件 (滚动保留策略)
///
/// # 参数
/// * `log_dir` - 日志存放目录
/// * `max_retention_days` - 最长保留天数 (超过则删除，如 7 天)
/// * `max_files` - 最大保留文件个数 (超过则删除最旧的，如 10 个)
pub fn clean_expired_logs(log_dir: &Path, max_retention_days: u64, max_files: usize) {
    let read_dir = match fs::read_dir(log_dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };

    let mut log_files = Vec::new();
    let now = SystemTime::now();
    let max_duration = Duration::from_secs(max_retention_days * 86400);

    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.is_file() {
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if (file_name.starts_with("smalux.log") || file_name.ends_with(".log"))
                && let Ok(metadata) = entry.metadata()
            {
                let modified = metadata.modified().unwrap_or(now);
                log_files.push((path, modified));
            }
        }
    }

    // 按修改时间从新到旧排序
    log_files.sort_by_key(|b| std::cmp::Reverse(b.1));

    // 1. 删除超出最大保留数量的旧文件
    if log_files.len() > max_files {
        for (path, _) in log_files.iter().skip(max_files) {
            let _ = fs::remove_file(path);
        }
        log_files.truncate(max_files);
    }

    // 2. 删除超出保留天数的过期文件
    for (path, modified) in log_files {
        if let Ok(age) = now.duration_since(modified)
            && age > max_duration
        {
            let _ = fs::remove_file(path);
        }
    }
}

/// 初始化全局 tracing 系统 (标准控制台 + 本地按天滚动持久化 + UI 事件层)
///
/// # 参数
/// * `app_name` - 应用名称 (用于日志目录命名)
/// * `custom_log_dir` - 可选的自定义日志存储目录
pub fn init_tracing(
    app_name: &str,
    custom_log_dir: Option<PathBuf>,
) -> anyhow::Result<TracingGuard> {
    let log_dir = custom_log_dir.unwrap_or_else(|| get_default_log_dir(app_name));
    let _ = fs::create_dir_all(&log_dir);

    // 自动清理 7 天前或超过 15 个的旧日志
    clean_expired_logs(&log_dir, 7, 15);

    // 1. 配置文件按天滚动写入 (Rolling File Appender)
    let file_appender = tracing_appender::rolling::daily(&log_dir, "smalux.log");
    let (non_blocking_file, file_guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(false)
        .with_writer(non_blocking_file);

    // 2. 配置标准终端控制台输出 (带 ANSI 彩色)
    let stdout_layer = fmt::layer()
        .with_ansi(true)
        .with_target(true);

    // 3. 配置 UI 调试面板实时捕获层
    let ui_layer = UiLogLayer::new(get_global_log_buffer());

    // 4. 环境过滤 (支持 RUST_LOG 动态控制，默认 smalux/smagical 开头包为 debug，其余为 info)
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("info,smagical_ui=debug,smagical_core=debug,smagical_debug=debug")
    });

    // 5. 组装全局 Registry
    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .with(file_layer)
        .with(ui_layer);

    // 设置为全局默认订阅器 (允许被调用一次)
    let _ = tracing::subscriber::set_global_default(subscriber);

    tracing::info!(
        target: "smagical_debug",
        "全局 Tracing 日志系统初始化就绪，本地文件日志输出至: {}",
        log_dir.display()
    );

    Ok(TracingGuard {
        _file_guard: file_guard,
    })
}
