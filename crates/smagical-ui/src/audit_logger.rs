//! 会话操作安全审计日志服务 (Session Security Audit Logger)。
//!
//! # 架构职责
//! 提供面向企业级运维与等保合规要求的操作审计追踪机制。
//! 当用户在设置中心启用“会话操作安全审计”(`session_audit_logging`)时：
//! 1. **全生命周期捕获**：透明捕获 SSH/本地终端会话的建立、关闭、指令片段注入、剪贴板粘贴与高危指令预警事件；
//! 2. **零 UI 阻塞写入**：通过 Tokio `spawn_blocking` 工作池在后台独立线程执行文件追加写操作，对主事件循环实现绝对的 0ms 开销；
//! 3. **按日滚动文件存储**：自动根据当前本地时间按 `audit_YYYY-MM-DD.log` 进行日维度物理文件切分归档；
//! 4. **跨平台目录自适应**：
//!    - Windows: `%APPDATA%\smalux\audit\audit_{date}.log`
//!    - Linux/macOS: `~/.smalux/audit/audit_{date}.log`

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

/// 全局会话审计功能启用开关。
///
/// 使用原子布尔变量存储，支持跨线程极速无锁读取，在主 UI 线程与 Tokio 协程中均可安全高频查询。
/// - `true`: 开启审计日志写入；
/// - `false`: 忽略所有审计写入调用，保持零磁盘 I/O。
static IS_AUDIT_LOGGING_ENABLED: AtomicBool = AtomicBool::new(false);

/// 设置会话审计日志功能的全局启用状态。
///
/// 通常由冷启动读取持久化配置或前端设置中心 `on_change_session_audit_logging` 变更时调用。
///
/// # 参数
/// - `enabled`: `true` 表示开启审计并落盘；`false` 表示关闭审计。
///
/// # 线程安全性
/// 内部使用 `Ordering::SeqCst` 强顺序原子写，确保所有线程立即可见最新状态。
pub fn set_audit_enabled(enabled: bool) {
    IS_AUDIT_LOGGING_ENABLED.store(enabled, Ordering::SeqCst);
    tracing::info!(target: "smagical_ui::audit", "会话操作安全审计日志已{}", if enabled { "开启" } else { "关闭" });
}

/// 查询当前全局会话审计日志功能是否处于启用状态。
///
/// # 返回值
/// - `true`: 审计已开启；
/// - `false`: 审计已关闭。
///
/// # 性能说明
/// 采用 `Ordering::SeqCst` 读取静态原子变量，耗时约为几个 CPU 周期，无互斥锁或系统调用开销。
pub fn is_audit_enabled() -> bool {
    IS_AUDIT_LOGGING_ENABLED.load(Ordering::SeqCst)
}

/// 解析并获取持久化审计日志的存储根目录。
///
/// 遵循操作系统的通用应用数据目录规范：
/// 1. 优先尝试系统推荐的本地应用数据目录：
///    - Windows: `C:\Users\<User>\AppData\Local\smagical\smalux\audit`
///    - Linux: `~/.local/share/smalux/audit`
///    - macOS: `~/Library/Application Support/com.smagical.smalux/audit`
/// 2. 备用尝试用户主目录：`~/.smalux/audit`
/// 3. 若均无法创建，回退至当前工作目录相对路径 `audit/`。
///
/// # 返回值
/// 确保已成功创建对应父级目录的 `PathBuf`。
pub fn get_audit_log_dir() -> PathBuf {
    // 方案一：标准 XDG / Windows LocalAppData
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "smagical", "smalux") {
        let dir = proj_dirs.data_local_dir().join("audit");
        if fs::create_dir_all(&dir).is_ok() {
            return dir;
        }
    }

    // 方案二：用户 Home 目录下隐藏配置夹
    if let Some(user_dirs) = directories::UserDirs::new() {
        let dir = user_dirs.home_dir().join(".smalux").join("audit");
        if fs::create_dir_all(&dir).is_ok() {
            return dir;
        }
    }

    // 方案三：当前进程所在路径相对目录兜底
    let fallback = PathBuf::from("audit");
    let _ = fs::create_dir_all(&fallback);
    fallback
}

/// 异步记录一条格式化的会话操作安全审计事件。
///
/// # 参数
/// - `session_id`: 触发该事件的终端会话唯一标识符 (例如 `"session-1"`, `"local-2"`)；
/// - `host_info`: 关联主机的描述信息 (例如 `"Prod-Gateway(192.168.1.10:22)"`, `"Local(cmd.exe)"`)；
/// - `action`: 审计操作类型标签，推荐使用以下标准大写词汇：
///   - `"SESSION_OPENED"`: 建立新会话
///   - `"SESSION_CLOSED"`: 关闭/注销会话
///   - `"EXECUTE_SNIPPET"`: 执行快捷代码片段或 AI 建议命令
///   - `"TERMINAL_PASTE"`: 剪贴板内容写入终端
///   - `"WARN_HIGH_RISK_COMMAND"`: 拦截或预警高危破坏性指令
///   - `"WARN_HIGH_RISK_PASTE"`: 预警高危粘贴内容
/// - `details`: 详细的指令内容、参数、风险判定原因或附加元数据。
///
/// # 执行机制
/// 1. 首先极速检查 `is_audit_enabled()`，若未开启直接返回，0 额外计算开销；
/// 2. 若已开启，将参数转为所有权类型并分发给 Tokio 运行时；
/// 3. 在 `spawn_blocking` 工作池内获取精确本地时间戳（毫秒精度），组装标准化单行日志，追加写盘；
/// 4. 彻底杜绝主 UI 线程因机械硬盘写延迟或防病毒软件扫描导致的掉帧和假死。
pub fn record_audit_event(session_id: &str, host_info: &str, action: &str, details: &str) {
    if !is_audit_enabled() {
        return;
    }

    let s_id = session_id.to_string();
    let h_info = host_info.to_string();
    let act = action.to_string();
    let det = details.to_string();

    crate::async_util::spawn_async(async move {
        tokio::task::spawn_blocking(move || {
            let now = chrono::Local::now();
            let date_str = now.format("%Y-%m-%d").to_string();
            let time_str = now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();

            let log_dir = get_audit_log_dir();
            let log_file_path = log_dir.join(format!("audit_{}.log", date_str));

            // 格式: [时间] [动作类型] [会话ID] [主机] 详细操作/命令
            let log_line = format!(
                "[{}] [{}] [session:{}] [host:{}] {}\n",
                time_str, act, s_id, h_info, det
            );

            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_file_path) {
                let _ = file.write_all(log_line.as_bytes());
            }
        });
    });
}
