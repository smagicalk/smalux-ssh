//! 右侧伴生工具栏抽屉生命周期治理、惰性加载、终端焦点同步与每机独立会话服务。
//!
//! 核心设计规范：
//! 1. 宿主从属生命周期 (Host-Scoped Cascading)：右侧栏完全服务于当前终端；终端销毁时伴生资源释放（FollowApp 隧道例外）。
//! 2. 惰性加载与关即停 (Lazy & Ephemeral)：抽屉打开时按需启动探针；抽屉关闭或最小化时唯独实时监控停止采样，其余后台长任务继续。
//! 3. 独立 AI 会话隔离 (Per-Host AI Isolation)：每台主机拥有完全独立的上下文会话流与草稿，互不串线，切终端无缝热置换。
//! 4. 主线程零竞态治理 (Main-Thread Direct Dispatch)：规避跨线程锁竞争与 Send 限制，与 Slint UI 运行循环同频。

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use slint::{ComponentHandle, Model, ModelRc};

use crate::common::{to_model_rc, ToSharedString};
use crate::generated::{
    AiBridge, AiChatMessage, AiHistorySession, AppWindow, FilesBridge, MonitorBridge, SettingsBridge,
    SnippetsBridge, SystemMetricsData, TerminalBridge, TmuxSessionItem, TunnelsBridge, WindowBridge,
};
use crate::handlers::AppContext;
use crate::monitor::IntoSlintMetrics;

/// AI 历史会话归档快照实体
///
/// 当用户清空或归档某台主机的当前对话时，历史记录保存在本实体中供随时调出查阅。
#[derive(Clone)]
pub(crate) struct ArchivedAiSession {
    /// 归档会话全局唯一 UUID
    pub(crate) id: String,
    /// 依据用户首轮提问智能提取的会话摘要标题
    pub(crate) title: String,
    /// 会话归属的主机唯一标识符
    pub(crate) host_id: String,
    /// 会话归属的主机显示名称
    pub(crate) host_name: String,
    /// 包含系统思考过程、审计建议与建议指令的完整消息历史切片
    pub(crate) messages: Vec<AiChatMessage>,
    /// 会话最后一次更新的本地时间字符串
    pub(crate) updated_time: String,
}

/// 单台主机独立的 AI 运维伴生会话状态实体 (Per-Host AI Isolation)
///
/// # 核心隔离保证
/// - 每个远程 SSH 会话或本地控制台拥有独立的对话流、输入草稿和参数模型；
/// - 切换终端选项卡时，伴生 AI 对话流瞬时无缝热置换，互不干扰且无串线泄露风险。
#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct HostAiSession {
    /// 关联的主机唯一标识符 (Host ID)
    pub(crate) host_id: String,
    /// 关联的主机展示名称
    pub(crate) host_name: String,
    /// 当前处于活动态的消息列表
    pub(crate) messages: Vec<AiChatMessage>,
    /// 用户在输入框中尚未发送的输入草稿内容 (切终端切回时自动恢复)
    pub(crate) draft_input: String,
    /// 该主机专属选定的大语言模型名称 (如 `"deepseek-reasoner"`)
    pub(crate) selected_model: String,
    /// 深度思考推理预算 (如 `"深度思考 (CoT)"`)
    pub(crate) thinking_degree: String,
    /// 预设的运维安全策略模板 (如 `"只读巡检"`)
    pub(crate) audit_policy: String,
    /// 指令下发审核模式 (如 `"自动"`)
    pub(crate) audit_mode: String,
    /// 风险阻断级别 (如 `"安全"`)
    pub(crate) auto_audit_level: String,
    /// 标志当前是否正在流式接收大模型 Token 生成响应
    pub(crate) is_generating: bool,
    /// 线程安全原子取消标志，当用户点击“停止生成”或关闭抽屉时置为 true 中断 HTTP 流
    pub(crate) abort_flag: Arc<AtomicBool>,
}

impl HostAiSession {
    /// 初始化单台主机的专属伴生会话实体
    ///
    /// # 处理流程
    /// 1. 解析目标主机展示名称；
    /// 2. 自动构建首条带有主机上下文挂接声明的初始欢迎消息；
    /// 3. 设置默认推理参数（DeepSeek 模型、深度思考模式、安全审计策略）。
    ///
    /// # 参数
    /// - `host_id`: 目标主机唯一 ID；
    /// - `host_name`: 目标主机展示名称。
    pub(crate) fn new(host_id: &str, host_name: &str) -> Self {
        let display_name = if host_name.is_empty() { "当前主机" } else { host_name };
        let initial_msg = AiChatMessage {
            id: format!("init-{}", host_id).into(),
            sender: "assistant".into(),
            content: format!(
                "你好！我是针对主机 [{}] 的专属运维副驾驶。当前会话与该主机严格绑定，已就绪为您分析系统日志、排查高危告警或生成操作维护命令。",
                display_name
            ).into(),
            thinking_content: format!("已成功挂接当前终端上下文。\n目标主机: {}\n标识: {}\n数据隔离状态: 已开启独立隔离会话", display_name, host_id).into(),
            is_thinking_expanded: false,
            suggested_cmd: "".into(),
            cmd_risk_level: "low".into(),
            audit_status: "approved".into(),
            audit_reason: "".into(),
            timestamp: "刚刚".into(),
        };

        Self {
            host_id: host_id.to_string(),
            host_name: host_name.to_string(),
            messages: vec![initial_msg],
            draft_input: String::new(),
            selected_model: "DeepSeek-R1".into(),
            thinking_degree: "深度思考 (CoT)".into(),
            audit_policy: "只读巡检".into(),
            audit_mode: "自动".into(),
            auto_audit_level: "安全".into(),
            is_generating: false,
            abort_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// AI 消息持久化实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StoredAiMessage {
    pub(crate) id: String,
    pub(crate) sender: String,
    pub(crate) content: String,
    pub(crate) thinking_content: String,
    pub(crate) is_thinking_expanded: bool,
    pub(crate) suggested_cmd: String,
    pub(crate) cmd_risk_level: String,
    pub(crate) audit_status: String,
    pub(crate) audit_reason: String,
    pub(crate) timestamp: String,
}

impl From<&AiChatMessage> for StoredAiMessage {
    fn from(m: &AiChatMessage) -> Self {
        Self {
            id: m.id.to_string(),
            sender: m.sender.to_string(),
            content: m.content.to_string(),
            thinking_content: m.thinking_content.to_string(),
            is_thinking_expanded: m.is_thinking_expanded,
            suggested_cmd: m.suggested_cmd.to_string(),
            cmd_risk_level: m.cmd_risk_level.to_string(),
            audit_status: m.audit_status.to_string(),
            audit_reason: m.audit_reason.to_string(),
            timestamp: m.timestamp.to_string(),
        }
    }
}

impl From<&StoredAiMessage> for AiChatMessage {
    fn from(m: &StoredAiMessage) -> Self {
        Self {
            id: m.id.to_shared(),
            sender: m.sender.to_shared(),
            content: m.content.to_shared(),
            thinking_content: m.thinking_content.to_shared(),
            is_thinking_expanded: m.is_thinking_expanded,
            suggested_cmd: m.suggested_cmd.to_shared(),
            cmd_risk_level: m.cmd_risk_level.to_shared(),
            audit_status: m.audit_status.to_shared(),
            audit_reason: m.audit_reason.to_shared(),
            timestamp: m.timestamp.to_shared(),
        }
    }
}

/// AI 历史归档会话持久化实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StoredArchivedSession {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) host_id: String,
    pub(crate) host_name: String,
    pub(crate) messages: Vec<StoredAiMessage>,
    pub(crate) updated_time: String,
}

impl From<&ArchivedAiSession> for StoredArchivedSession {
    fn from(a: &ArchivedAiSession) -> Self {
        Self {
            id: a.id.clone(),
            title: a.title.clone(),
            host_id: a.host_id.clone(),
            host_name: a.host_name.clone(),
            messages: a.messages.iter().map(StoredAiMessage::from).collect(),
            updated_time: a.updated_time.clone(),
        }
    }
}

impl From<StoredArchivedSession> for ArchivedAiSession {
    fn from(a: StoredArchivedSession) -> Self {
        Self {
            id: a.id,
            title: a.title,
            host_id: a.host_id,
            host_name: a.host_name,
            messages: a.messages.iter().map(AiChatMessage::from).collect(),
            updated_time: a.updated_time,
        }
    }
}

/// 单台主机独立的 AI 会话持久化实体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StoredHostAiSession {
    pub(crate) host_id: String,
    pub(crate) host_name: String,
    pub(crate) messages: Vec<StoredAiMessage>,
    pub(crate) draft_input: String,
    pub(crate) selected_model: String,
    pub(crate) thinking_degree: String,
    pub(crate) audit_policy: String,
    pub(crate) audit_mode: String,
    pub(crate) auto_audit_level: String,
}

impl From<&HostAiSession> for StoredHostAiSession {
    fn from(s: &HostAiSession) -> Self {
        Self {
            host_id: s.host_id.clone(),
            host_name: s.host_name.clone(),
            messages: s.messages.iter().map(StoredAiMessage::from).collect(),
            draft_input: s.draft_input.clone(),
            selected_model: s.selected_model.clone(),
            thinking_degree: s.thinking_degree.clone(),
            audit_policy: s.audit_policy.clone(),
            audit_mode: s.audit_mode.clone(),
            auto_audit_level: s.auto_audit_level.clone(),
        }
    }
}

impl StoredHostAiSession {
    pub(crate) fn into_host_ai_session(self) -> HostAiSession {
        HostAiSession {
            host_id: self.host_id,
            host_name: self.host_name,
            messages: self.messages.iter().map(AiChatMessage::from).collect(),
            draft_input: self.draft_input,
            selected_model: self.selected_model,
            thinking_degree: self.thinking_degree,
            audit_policy: self.audit_policy,
            audit_mode: self.audit_mode,
            auto_audit_level: self.auto_audit_level,
            is_generating: false,
            abort_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// AI 全量会话与归档持久化快照
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct StoredAiData {
    pub(crate) history_archives: Vec<StoredArchivedSession>,
    pub(crate) active_sessions: HashMap<String, StoredHostAiSession>,
}

/// 获取 AI 会话历史持久化存储文件路径
pub(crate) fn get_ai_data_path() -> PathBuf {
    let sqlite_path = smagical_storage::seaorm::get_default_sqlite_path();
    if let Some(parent) = sqlite_path.parent() {
        let _ = std::fs::create_dir_all(parent);
        parent.join("ai_sessions.json")
    } else {
        PathBuf::from("ai_sessions.json")
    }
}

/// 从物理磁盘加载 AI 会话与历史归档数据
pub(crate) fn load_ai_data_from_disk() -> StoredAiData {
    let path = get_ai_data_path();
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(data) = serde_json::from_str::<StoredAiData>(&content) {
                tracing::info!(
                    target: "smagical_ui::ai",
                    "已从磁盘恢复 AI 会话数据 (历史归档: {}, 活跃会话: {})",
                    data.history_archives.len(),
                    data.active_sessions.len()
                );
                return data;
            }
        }
    }
    StoredAiData::default()
}

/// 异步非阻塞持久化保存 AI 会话与历史归档至磁盘
pub(crate) fn save_ai_data_to_disk_async(data: StoredAiData) {
    crate::async_util::spawn_async(async move {
        let path = get_ai_data_path();
        if let Ok(json_str) = serde_json::to_string_pretty(&data) {
            if let Err(e) = tokio::fs::write(&path, json_str).await {
                tracing::warn!(target: "smagical_ui::ai", "持久化 AI 会话数据失败: {}", e);
            }
        }
    });
}

/// 同步阻塞持久化保存 AI 会话与历史归档至磁盘 (用于进程退出清理)
pub(crate) fn save_ai_data_to_disk_sync(data: &StoredAiData) {
    let path = get_ai_data_path();
    if let Ok(json_str) = serde_json::to_string_pretty(data) {
        if let Err(e) = std::fs::write(&path, json_str) {
            tracing::warn!(target: "smagical_ui::ai", "同步持久化 AI 会话数据失败: {}", e);
        }
    }
}

/// 提取当前抽屉状态中的全量 AI 会话持久化快照
pub(crate) fn snapshot_stored_ai_data(state: &RightDrawerState) -> StoredAiData {
    StoredAiData {
        history_archives: state.history_archives.iter().map(StoredArchivedSession::from).collect(),
        active_sessions: state.ai_sessions.iter()
            .map(|(k, v)| (k.clone(), StoredHostAiSession::from(v)))
            .collect(),
    }
}

/// 右侧伴生抽屉主线程运行时状态中枢
///
/// 专用于 UI 线程直连访问，完全消除锁竞争，管理多机隔离的 AI 会话集与惰性性能采样探针。
pub(crate) struct RightDrawerState {
    /// 全局应用上下文句柄弱引用
    pub(crate) ctx: Option<AppContext>,
    /// 每台主机专属的独立 AI 会话映射表，键为 `host_id`
    pub(crate) ai_sessions: HashMap<String, HostAiSession>,
    /// 会话历史归档仓库
    pub(crate) history_archives: Vec<ArchivedAiSession>,
    /// 实时监控系统指标采样的定时器句柄（开抽屉即启，关抽屉即停）
    pub(crate) monitor_timer: Option<slint::Timer>,
    /// tmux 会话列表定时轮询句柄（开抽屉即启，关抽屉即停）
    pub(crate) tmux_timer: Option<slint::Timer>,
    /// 原子防重入标志：防止上一次异步 SSH tmux 探活/拉取未返回前开启重叠查询
    pub(crate) is_tmux_busy: Arc<AtomicBool>,
    /// 周期性采样的时间序列阶段步进计数器
    pub(crate) sampling_phase: usize,
    /// 当前抽屉激活绑定的主机 ID
    pub(crate) current_host_id: String,
    /// Linux 远程探针指标采集引擎（解析 `/proc/stat`, `/proc/meminfo`, `/proc/net/dev` 等）
    pub(crate) linux_sampler: Arc<crate::monitor::LinuxMetricsSampler>,
    /// 原子防重入标志：防止上一次异步 SSH 性能拉取未返回前开启重叠采样
    pub(crate) is_sampling_busy: Arc<AtomicBool>,
}

impl Default for RightDrawerState {
    fn default() -> Self {
        Self {
            ctx: None,
            ai_sessions: HashMap::new(),
            history_archives: Vec::new(),
            monitor_timer: None,
            tmux_timer: None,
            is_tmux_busy: Arc::new(AtomicBool::new(false)),
            sampling_phase: 0,
            current_host_id: String::new(),
            linux_sampler: Arc::new(crate::monitor::LinuxMetricsSampler::default()),
            is_sampling_busy: Arc::new(AtomicBool::new(false)),
        }
    }
}

thread_local! {
    static DRAWER_STATE: RefCell<RightDrawerState> = RefCell::new(RightDrawerState::default());
}

/// 卸载右侧伴生工具绑定的 Slint ModelRc 数据模型，释放 UI 堆内存
pub(crate) fn unload_right_tool_models(window: &AppWindow, tool_id: &str) {
    match tool_id {
        "sftp" => {
            let fb = window.global::<FilesBridge>();
            fb.set_remote_files(ModelRc::default());
            fb.set_transfer_tasks(ModelRc::default());
            tracing::debug!(target: "smalux::lifecycle", "已释放右侧 SFTP 抽屉 Slint 远程文件与传输任务模型");
        }
        "ai" => {
            let ab = window.global::<AiBridge>();
            ab.set_messages(ModelRc::default());
            ab.set_history_sessions(ModelRc::default());
            tracing::debug!(target: "smalux::lifecycle", "已释放右侧 AI 助手抽屉 Slint 消息与历史模型");
        }
        "tunnel" => {
            let tb = window.global::<TunnelsBridge>();
            tb.set_host_tunnels(ModelRc::default());
            tracing::debug!(target: "smalux::lifecycle", "已释放右侧 Tunnels 伴生抽屉 Slint 隧道模型");
        }
        "tmux" => {
            DRAWER_STATE.with(|state_cell| {
                let mut state = state_cell.borrow_mut();
                stop_tmux_polling_inner(window, &mut state);
            });
            tracing::debug!(target: "smalux::lifecycle", "已释放右侧 Tmux 伴生抽屉 Slint 会话状态与轮询");
        }
        "snippets" => {
            let sb = window.global::<SnippetsBridge>();
            sb.set_quick_cmds(ModelRc::default());
            tracing::debug!(target: "smalux::lifecycle", "已释放右侧 Snippets 伴生抽屉 Slint 快捷指令模型");
        }
        "monitor" => {
            DRAWER_STATE.with(|state_cell| {
                let mut state = state_cell.borrow_mut();
                stop_monitor_sampling_inner(window, &mut state);
            });
            tracing::debug!(target: "smalux::lifecycle", "已停止并清空 Monitor 伴生抽屉监控探针与采样");
        }
        _ => {}
    }
}

/// 重新从纯 Rust 内存状态回填右侧伴生工具 Slint 模型 (微秒级瞬时恢复)
pub(crate) fn load_right_tool_models(window: &AppWindow, tool_id: &str) {
    DRAWER_STATE.with(|state_cell| {
        let mut state = state_cell.borrow_mut();
        let term_b = window.global::<TerminalBridge>();
        let h_id = if !state.current_host_id.is_empty() {
            state.current_host_id.clone()
        } else {
            term_b.get_active_host_id().to_string()
        };
        let h_name = term_b.get_active_host_name().to_string();
        state.current_host_id = h_id.clone();

        if tool_id == "monitor" {
            stop_tmux_polling_inner(window, &mut state);
            start_monitor_sampling_inner(window, &mut state, &h_id, &h_name);
        } else {
            stop_monitor_sampling_inner(window, &mut state);
            if tool_id == "tmux" {
                start_tmux_polling_inner(window, &mut state, &h_id, &h_name);
            } else {
                stop_tmux_polling_inner(window, &mut state);
                if tool_id == "tunnel" {
                    if let Some(ref c) = state.ctx {
                        crate::handlers::tunnel_handlers::sync_ui_host_tunnels(window, c);
                    }
                } else if tool_id == "ai" {
                    let history_ui_list: Vec<AiHistorySession> = state.history_archives.iter()
                        .map(|a| AiHistorySession {
                            id: a.id.to_shared(),
                            title: a.title.to_shared(),
                            message_count: a.messages.len() as i32,
                            updated_time: a.updated_time.to_shared(),
                        })
                        .collect();
                    window.global::<AiBridge>().set_history_sessions(to_model_rc(history_ui_list));
                    sync_ai_for_host_inner(window, &mut state, &h_id, &h_name);
                } else if tool_id == "sftp" {
                    if let Some(ref c) = state.ctx {
                        crate::handlers::file_handlers::sync_sftp_drawer_for_host(window, c, &h_id, &h_name);
                    }
                } else if tool_id == "snippets" {
                    if let Some(ref c) = state.ctx {
                        crate::handlers::snippet_handlers::sync_ui_snippets(window, c);
                    }
                }
            }
        }
    });
}

/// 注册右侧伴生工具栏全套生命周期治理与 UI 回调处理器
///
/// 挂载涵盖：
/// 1. 抽屉打开/关闭与活动标签切换（Monitor 性能监控 / SFTP 文件传输 / Tunnel 专属转发 / AI 副驾驶）；
/// 2. 惰性按需探针管理（开抽屉启动 2 秒定时间歇采样，关抽屉或失焦立即休眠探针释放服务器资源）；
/// 3. 每机独立 AI 会话流转、流式代码块解析与高危命令语法审计；
/// 4. SFTP 快速传输抽屉绑定当前活动终端主机。
///
/// # 参数
/// - `window`: Slint 顶级应用主窗口；
/// - `ctx`: 应用程序全局上下文引用。
pub(crate) fn register_right_drawer_handlers(window: &AppWindow, ctx: &AppContext) {
    let loaded = load_ai_data_from_disk();
    DRAWER_STATE.with(|state_cell| {
        let mut state = state_cell.borrow_mut();
        state.ctx = Some(ctx.clone());
        state.history_archives = loaded.history_archives.into_iter().map(ArchivedAiSession::from).collect();
        for (k, v) in loaded.active_sessions {
            state.ai_sessions.insert(k, v.into_host_ai_session());
        }

        let history_ui_list: Vec<AiHistorySession> = state.history_archives.iter()
            .map(|a| AiHistorySession {
                id: a.id.to_shared(),
                title: a.title.to_shared(),
                message_count: a.messages.len() as i32,
                updated_time: a.updated_time.to_shared(),
            })
            .collect();
        window.global::<AiBridge>().set_history_sessions(to_model_rc(history_ui_list));
    });

    // -------------------------------------------------------------------------
    // 1. 挂载 WindowBridge 抽屉切换与开闭回调
    // -------------------------------------------------------------------------
    {
        let w_weak = window.as_weak();
        window.global::<WindowBridge>().on_switch_right_tool(move |tool_id| {
            if let Some(w) = w_weak.upgrade() {
                let wb = w.global::<WindowBridge>();
                let prev_tool = wb.get_active_right_tool().to_string();
                if prev_tool != tool_id.as_str() {
                    unload_right_tool_models(&w, &prev_tool);
                }

                wb.set_active_right_tool(tool_id.clone());
                wb.set_is_right_drawer_open(true);

                DRAWER_STATE.with(|state_cell| {
                    let state = state_cell.borrow();
                    if let Some(ref c) = state.ctx {
                        c.core_state.toggle_right_panel(&tool_id);
                    }
                });

                load_right_tool_models(&w, &tool_id);
            }
        });
    }

    {
        let w_weak = window.as_weak();
        window.global::<WindowBridge>().on_toggle_right_drawer(move || {
            if let Some(w) = w_weak.upgrade() {
                let wb = w.global::<WindowBridge>();
                let is_open = wb.get_is_right_drawer_open();
                let tool_id = wb.get_active_right_tool().to_string();

                DRAWER_STATE.with(|state_cell| {
                    let state = state_cell.borrow();
                    if let Some(ref c) = state.ctx {
                        c.core_state.right_panels().write().unwrap().set_drawer_open(is_open);
                    }
                });

                if !is_open {
                    unload_right_tool_models(&w, &tool_id);
                } else {
                    load_right_tool_models(&w, &tool_id);
                }
            }
        });
    }

    // -------------------------------------------------------------------------
    // 2. 挂载 AiBridge 交互回调 (支持每机独立历史、流式回复模拟与命令注入)
    // -------------------------------------------------------------------------
    {
        let w_weak = window.as_weak();
        let ai_bridge = window.global::<AiBridge>();

        // 2.1 发送用户消息并触发 AI 推理
        let w_weak_send = w_weak.clone();
        ai_bridge.on_send_message(move |content, model, thinking, audit| {
            if let Some(w) = w_weak_send.upgrade() {
                let model_str = model.to_string();
                let thinking_str = thinking.to_string();
                let audit_str = audit.to_string();

                let (h_id, h_name, auto_audit_level, abort_flag, ai_msg_id) = DRAWER_STATE.with(|state_cell| {
                    let mut state = state_cell.borrow_mut();
                    let raw_id = state.current_host_id.clone();
                    let active_term_id = w.global::<TerminalBridge>().get_active_host_id().to_string();
                    let active_term_name = w.global::<TerminalBridge>().get_active_host_name().to_string();

                    let h_id = if !raw_id.is_empty() {
                        raw_id
                    } else if !active_term_id.is_empty() {
                        active_term_id
                    } else {
                        "default".to_string()
                    };

                    let h_name = if !active_term_name.is_empty() {
                        active_term_name
                    } else {
                        "当前终端".to_string()
                    };

                    state.current_host_id = h_id.clone();

                    let user_msg = AiChatMessage {
                        id: format!("usr-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()).into(),
                        sender: "user".into(),
                        content: content.clone(),
                        thinking_content: "".into(),
                        is_thinking_expanded: false,
                        suggested_cmd: "".into(),
                        cmd_risk_level: "low".into(),
                        audit_status: "approved".into(),
                        audit_reason: "".into(),
                        timestamp: "刚刚".into(),
                    };

                    let ai_msg_id = format!("ai-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis());
                    let placeholder_ai_msg = AiChatMessage {
                        id: ai_msg_id.to_shared(),
                        sender: "assistant".into(),
                        content: "".into(),
                        thinking_content: "".into(),
                        is_thinking_expanded: true,
                        suggested_cmd: "".into(),
                        cmd_risk_level: "low".into(),
                        audit_status: "pending".into(),
                        audit_reason: "".into(),
                        timestamp: "思考中...".into(),
                    };

                    let session = state.ai_sessions.entry(h_id.clone()).or_insert_with(|| HostAiSession::new(&h_id, &h_name));
                    session.messages.push(user_msg);
                    session.messages.push(placeholder_ai_msg);
                    session.selected_model = model_str.clone();
                    session.thinking_degree = thinking_str.clone();
                    session.audit_policy = audit_str.clone();
                    match audit_str.as_str() {
                        "纯人工" => { session.audit_mode = "人工".into(); }
                        "只读巡检" => { session.audit_mode = "自动".into(); session.auto_audit_level = "安全".into(); }
                        "辅助运维" => { session.audit_mode = "自动".into(); session.auto_audit_level = "警告".into(); }
                        "免审直达" => { session.audit_mode = "自动".into(); session.auto_audit_level = "危险".into(); }
                        "AI自审" => { session.audit_mode = "AI".into(); }
                        _ => { session.audit_mode = audit_str.clone(); }
                    }
                    session.is_generating = true;
                    session.abort_flag.store(false, Ordering::SeqCst);
                    let abort_flag = Arc::clone(&session.abort_flag);

                    let auto_audit_level = session.auto_audit_level.clone();
                    sync_ai_for_host_inner(&w, &mut state, &h_id, &h_name);
                    (h_id, h_name, auto_audit_level, Some(abort_flag), ai_msg_id)
                });

                if let Some(abort_flag) = abort_flag {
                    w.global::<AiBridge>().set_input_text("".into());

                    // 从 Settings 读取端点配置
                    let sb = w.global::<SettingsBridge>();
                    let ep_base_url = sb.get_setting_ai_base_url().trim().to_string();
                    let ep_api_key = sb.get_setting_ai_api_key().trim().to_string();
                    let ep_temp = sb.get_setting_ai_temperature();
                    let ep_headers = sb.get_setting_ai_custom_headers().trim().to_string();
                    let ep_timeout = sb.get_setting_ai_timeout_secs() as u64;
                    let ep_system_prompt = sb.get_setting_ai_system_prompt().trim().to_string();
                    let ep_model = if !model_str.is_empty() { model_str.clone() } else { sb.get_setting_ai_model().trim().to_string() };

                    let w_weak_stream = w.as_weak();
                    let content_str = content.to_string();
                    let h_id_task = h_id.clone();
                    let h_name_task = h_name.clone();
                    let m_str = model_str.clone();
                    let t_str = thinking_str.clone();
                    let a_str = audit_str.clone();
                    let auto_lvl_str = auto_audit_level.clone();

                    let is_real_llm = !ep_api_key.is_empty() || ep_base_url.contains("localhost") || ep_base_url.contains("127.0.0.1") || ep_base_url.contains("11434");

                    crate::async_util::spawn_async(async move {
                        if abort_flag.load(Ordering::SeqCst) {
                            tracing::info!(target: "smagical_ui::ai", "AI 生成任务在启动前已被熔断取消");
                            return;
                        }

                        if is_real_llm {
                            // 1. 真实远端/本地大模型原生纯 Rust SSE 流式推理
                            let client_cfg = smagical_core::AiEndpointConfig {
                                base_url: ep_base_url,
                                api_key: ep_api_key,
                                model: ep_model.clone(),
                                temperature: ep_temp,
                                timeout_secs: if ep_timeout == 0 { 60 } else { ep_timeout },
                                custom_headers: if ep_headers.is_empty() { None } else { Some(ep_headers) },
                            };

                            let mut req_messages = Vec::new();
                            if !ep_system_prompt.is_empty() {
                                req_messages.push(smagical_core::AiChatMessage::system(ep_system_prompt));
                            }
                            req_messages.push(smagical_core::AiChatMessage::user(content_str.clone()));

                            let chat_req = smagical_core::AiChatRequest {
                                model: ep_model,
                                messages: req_messages,
                                stream: true,
                                temperature: Some(ep_temp),
                                max_tokens: None,
                            };

                            let client = match smagical_core::AiClient::new(client_cfg) {
                                Ok(c) => c,
                                Err(e) => {
                                    tracing::error!(target: "smagical_ui::ai", "创建 AI Client 失败: {:?}", e);
                                    return;
                                }
                            };

                            match client.stream_chat(chat_req).await {
                                Ok(mut rx) => {
                                    let mut full_content = String::new();
                                    let mut full_thinking = String::new();
                                    let mut is_completed = false;

                                    while let Some(chunk_res) = rx.recv().await {
                                        if abort_flag.load(Ordering::SeqCst) {
                                            tracing::info!(target: "smagical_ui::ai", "AI 流式生成已熔断取消");
                                            break;
                                        }

                                        match chunk_res {
                                            Ok(chunk) => {
                                                full_content.push_str(&chunk.delta_content);
                                                full_thinking.push_str(&chunk.delta_thinking);
                                                if chunk.is_final {
                                                    is_completed = true;
                                                }

                                                let extracted = smagical_core::extract_shell_command(&full_content).unwrap_or_default();
                                                let (risk, reason) = if !extracted.is_empty() {
                                                    smagical_core::assess_command_risk(&extracted)
                                                } else {
                                                    ("low", "")
                                                };

                                                let audit_status = match a_str.as_str() {
                                                    "纯人工" => "pending",
                                                    "只读巡检" => if risk == "low" { "approved" } else { "pending" },
                                                    "辅助运维" => if risk != "high" { "approved" } else { "pending" },
                                                    "免审直达" => "executed",
                                                    "AI自审" => "approved",
                                                    _ => "pending",
                                                };

                                                let is_exec = audit_status == "executed" && is_completed;
                                                let cmd_to_run = extracted.clone();

                                                let w_up = w_weak_stream.clone();
                                                let h_id_copy = h_id_task.clone();
                                                let h_name_copy = h_name_task.clone();
                                                let content_copy = full_content.clone();
                                                let thinking_copy = full_thinking.clone();
                                                let cmd_copy = extracted.clone();
                                                let msg_id_copy = ai_msg_id.clone();
                                                let status_copy = audit_status.to_string();
                                                let reason_copy = reason.to_string();
                                                let risk_copy = risk.to_string();

                                                let _ = slint::invoke_from_event_loop(move || {
                                                    DRAWER_STATE.with(|state_cell| {
                                                        let mut state = state_cell.borrow_mut();
                                                        if let Some(sess) = state.ai_sessions.get_mut(&h_id_copy) {
                                                            if let Some(msg) = sess.messages.iter_mut().find(|m| m.id == msg_id_copy) {
                                                                msg.content = content_copy.into();
                                                                msg.thinking_content = thinking_copy.into();
                                                                msg.suggested_cmd = cmd_copy.into();
                                                                msg.cmd_risk_level = risk_copy.into();
                                                                msg.audit_status = status_copy.into();
                                                                msg.audit_reason = reason_copy.into();
                                                                msg.timestamp = if is_completed { "刚刚".into() } else { "思考中...".into() };
                                                            }
                                                            if is_completed {
                                                                sess.is_generating = false;
                                                                let snapshot = snapshot_stored_ai_data(&state);
                                                                save_ai_data_to_disk_async(snapshot);
                                                            }
                                                        }

                                                        if let Some(w2) = w_up.upgrade() {
                                                            if state.current_host_id == h_id_copy || state.current_host_id.is_empty() || state.current_host_id == "default" {
                                                                let ai_b = w2.global::<AiBridge>();
                                                                let cur_msgs = ai_b.get_messages();
                                                                if let Some(sess) = state.ai_sessions.get(&h_id_copy) {
                                                                    if cur_msgs.row_count() == sess.messages.len() && !sess.messages.is_empty() {
                                                                        // 尾部原地灌入：直接更新末行数据，避免每次接收 Token 重构全局消息列表
                                                                        let last_idx = sess.messages.len() - 1;
                                                                        cur_msgs.set_row_data(last_idx, sess.messages[last_idx].clone());
                                                                        ai_b.set_is_generating(sess.is_generating);
                                                                    } else {
                                                                        sync_ai_for_host_inner(&w2, &mut state, &h_id_copy, &h_name_copy);
                                                                    }
                                                                }
                                                                if is_exec && !cmd_to_run.is_empty() {
                                                                    let cmd_with_nl = format!("{}\n", cmd_to_run);
                                                                    w2.global::<TerminalBridge>().invoke_send_snippet(cmd_with_nl.into());
                                                                }
                                                            }
                                                        }
                                                    });
                                                });
                                            }
                                            Err(err) => {
                                                tracing::warn!(target: "smagical_ui::ai", "流式接收异常: {:?}", err);
                                                let err_text = format!("流式生成异常中断: {}", err);
                                                let w_up = w_weak_stream.clone();
                                                let h_id_copy = h_id_task.clone();
                                                let h_name_copy = h_name_task.clone();
                                                let msg_id_copy = ai_msg_id.clone();
                                                let _ = slint::invoke_from_event_loop(move || {
                                                    DRAWER_STATE.with(|state_cell| {
                                                        let mut state = state_cell.borrow_mut();
                                                        if let Some(sess) = state.ai_sessions.get_mut(&h_id_copy) {
                                                            if let Some(msg) = sess.messages.iter_mut().find(|m| m.id == msg_id_copy) {
                                                                if msg.content.is_empty() {
                                                                    msg.content = err_text.into();
                                                                }
                                                                msg.timestamp = "异常中断".into();
                                                            }
                                                            sess.is_generating = false;
                                                            let snapshot = snapshot_stored_ai_data(&state);
                                                            save_ai_data_to_disk_async(snapshot);
                                                        }
                                                        if let Some(w2) = w_up.upgrade() {
                                                            sync_ai_for_host_inner(&w2, &mut state, &h_id_copy, &h_name_copy);
                                                        }
                                                    });
                                                });
                                                break;
                                            }
                                        }
                                    }
                                }
                                Err(err) => {
                                    tracing::error!(target: "smagical_ui::ai", "发起 AI 会话失败: {:?}", err);
                                    let err_text = format!("连接 AI 端点失败: {}\n请在「设置 - AI 助手」中检查 API 接口地址与 API Key 凭据配置。", err);
                                    let w_up = w_weak_stream.clone();
                                    let h_id_copy = h_id_task.clone();
                                    let h_name_copy = h_name_task.clone();
                                    let msg_id_copy = ai_msg_id.clone();
                                    let _ = slint::invoke_from_event_loop(move || {
                                        DRAWER_STATE.with(|state_cell| {
                                            let mut state = state_cell.borrow_mut();
                                            if let Some(sess) = state.ai_sessions.get_mut(&h_id_copy) {
                                                if let Some(msg) = sess.messages.iter_mut().find(|m| m.id == msg_id_copy) {
                                                    msg.content = err_text.into();
                                                    msg.timestamp = "连接失败".into();
                                                }
                                                sess.is_generating = false;
                                                let snapshot = snapshot_stored_ai_data(&state);
                                                save_ai_data_to_disk_async(snapshot);
                                            }
                                            if let Some(w2) = w_up.upgrade() {
                                                sync_ai_for_host_inner(&w2, &mut state, &h_id_copy, &h_name_copy);
                                            }
                                        });
                                    });
                                }
                            }
                        } else {
                            // 2. 本地高保真打字机流式输出模拟 (带完整的 DeepSeek-R1 思考链、建议命令提取与安全审核)
                            let (suggested_cmd, risk_level, audit_status, audit_reason, thinking_text, reply_text) =
                                generate_ai_devops_response(&h_name_task, &content_str, &m_str, &t_str, &a_str, &auto_lvl_str);

                            let is_executed = audit_status == "executed";
                            let cmd_to_run = suggested_cmd.clone();

                            // 2.1 逐字符流式输出思考过程 (CoT)
                            let mut current_thinking = String::new();
                            let thinking_chars: Vec<char> = thinking_text.chars().collect();
                            for chunk in thinking_chars.chunks(6) {
                                if abort_flag.load(Ordering::SeqCst) {
                                    return;
                                }
                                for c in chunk {
                                    current_thinking.push(*c);
                                }
                                let w_up = w_weak_stream.clone();
                                let h_id_copy = h_id_task.clone();
                                let h_name_copy = h_name_task.clone();
                                let thinking_copy = current_thinking.clone();
                                let msg_id_copy = ai_msg_id.clone();

                                let _ = slint::invoke_from_event_loop(move || {
                                    DRAWER_STATE.with(|state_cell| {
                                        let mut state = state_cell.borrow_mut();
                                        if let Some(sess) = state.ai_sessions.get_mut(&h_id_copy) {
                                            if let Some(msg) = sess.messages.iter_mut().find(|m| m.id == msg_id_copy) {
                                                msg.thinking_content = thinking_copy.into();
                                                msg.timestamp = "思考中...".into();
                                            }
                                        }
                                        if let Some(w2) = w_up.upgrade() {
                                            if state.current_host_id == h_id_copy || state.current_host_id.is_empty() || state.current_host_id == "default" {
                                                sync_ai_for_host_inner(&w2, &mut state, &h_id_copy, &h_name_copy);
                                            }
                                        }
                                    });
                                });
                                tokio::time::sleep(tokio::time::Duration::from_millis(18)).await;
                            }

                            // 2.2 逐字符流式输出正文回复并挂载命令
                            let mut current_reply = String::new();
                            let reply_chars: Vec<char> = reply_text.chars().collect();
                            let total_chunks = (reply_chars.len() + 4) / 5;
                            let mut chunk_idx = 0;

                            for chunk in reply_chars.chunks(5) {
                                if abort_flag.load(Ordering::SeqCst) {
                                    return;
                                }
                                for c in chunk {
                                    current_reply.push(*c);
                                }
                                chunk_idx += 1;
                                let is_final = chunk_idx >= total_chunks;

                                let w_up = w_weak_stream.clone();
                                let h_id_copy = h_id_task.clone();
                                let h_name_copy = h_name_task.clone();
                                let reply_copy = current_reply.clone();
                                let thinking_copy = thinking_text.clone();
                                let cmd_copy = if is_final { suggested_cmd.clone() } else { String::new() };
                                let risk_copy = risk_level.clone();
                                let status_copy = audit_status.clone();
                                let reason_copy = audit_reason.clone();
                                let msg_id_copy = ai_msg_id.clone();
                                let cmd_exec = cmd_to_run.clone();

                                let _ = slint::invoke_from_event_loop(move || {
                                    DRAWER_STATE.with(|state_cell| {
                                        let mut state = state_cell.borrow_mut();
                                        if let Some(sess) = state.ai_sessions.get_mut(&h_id_copy) {
                                            if let Some(msg) = sess.messages.iter_mut().find(|m| m.id == msg_id_copy) {
                                                msg.content = reply_copy.into();
                                                msg.thinking_content = thinking_copy.into();
                                                if is_final {
                                                    msg.suggested_cmd = cmd_copy.into();
                                                    msg.cmd_risk_level = risk_copy.into();
                                                    msg.audit_status = status_copy.into();
                                                    msg.audit_reason = reason_copy.into();
                                                    msg.timestamp = "刚刚".into();
                                                }
                                            }
                                            if is_final {
                                                sess.is_generating = false;
                                                let snapshot = snapshot_stored_ai_data(&state);
                                                save_ai_data_to_disk_async(snapshot);
                                            }
                                        }
                                        if let Some(w2) = w_up.upgrade() {
                                            if state.current_host_id == h_id_copy || state.current_host_id.is_empty() || state.current_host_id == "default" {
                                                sync_ai_for_host_inner(&w2, &mut state, &h_id_copy, &h_name_copy);
                                                if is_final && is_executed && !cmd_exec.is_empty() {
                                                    let cmd_with_nl = format!("{}\n", cmd_exec);
                                                    w2.global::<TerminalBridge>().invoke_send_snippet(cmd_with_nl.into());
                                                }
                                            }
                                        }
                                    });
                                });
                                tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
                            }
                        }
                    });
                }
            }
        });

        // 2.2 开启新会话 (旧的归档到历史会话，重置当前会话为全新输入)
        let w_weak_new = w_weak.clone();
        ai_bridge.on_new_session(move || {
            if let Some(w) = w_weak_new.upgrade() {
                DRAWER_STATE.with(|state_cell| {
                    let mut state = state_cell.borrow_mut();
                    let raw_id = state.current_host_id.clone();
                    let active_term_id = w.global::<TerminalBridge>().get_active_host_id().to_string();
                    let active_term_name = w.global::<TerminalBridge>().get_active_host_name().to_string();

                    let target_id = if !raw_id.is_empty() {
                        raw_id
                    } else if !active_term_id.is_empty() {
                        active_term_id
                    } else {
                        "default".to_string()
                    };

                    let h_name = if !active_term_name.is_empty() {
                        active_term_name
                    } else {
                        "当前终端".to_string()
                    };

                    // 1. 如果当前会话有用户提问，将其归档到历史会话
                    if let Some(current_sess) = state.ai_sessions.get(&target_id) {
                        let user_msgs: Vec<&AiChatMessage> = current_sess.messages.iter()
                            .filter(|m| m.sender == "user")
                            .collect();

                        if !user_msgs.is_empty() {
                            let first_prompt = user_msgs[0].content.to_string();
                            let title = if first_prompt.chars().count() > 16 {
                                format!("{}...", first_prompt.chars().take(16).collect::<String>())
                            } else {
                                first_prompt
                            };

                            let time_str = {
                                let now = std::time::SystemTime::now();
                                let s = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                                let h = ((s % 86400) / 3600 + 8) % 24;
                                let m = (s % 3600) / 60;
                                format!("{:02}:{:02}", h, m)
                            };

                            let archive = ArchivedAiSession {
                                id: format!("arch-{}", uuid::Uuid::new_v4().simple()),
                                title,
                                host_id: target_id.clone(),
                                host_name: h_name.clone(),
                                messages: current_sess.messages.clone(),
                                updated_time: time_str,
                            };
                            state.history_archives.insert(0, archive);
                        }
                    }

                    // 2. 同步更新 AiBridge 的历史会话列表
                    let history_ui_list: Vec<AiHistorySession> = state.history_archives.iter()
                        .map(|a| AiHistorySession {
                            id: a.id.to_shared(),
                            title: a.title.to_shared(),
                            message_count: a.messages.len() as i32,
                            updated_time: a.updated_time.to_shared(),
                        })
                        .collect();
                    w.global::<AiBridge>().set_history_sessions(to_model_rc(history_ui_list));

                    // 3. 重置当前主机/全局会话为全新会话
                    state.ai_sessions.insert(target_id.clone(), HostAiSession::new(&target_id, &h_name));
                    sync_ai_for_host_inner(&w, &mut state, &target_id, &h_name);
                    w.global::<AiBridge>().set_input_text("".into());
                    let snapshot = snapshot_stored_ai_data(&state);
                    save_ai_data_to_disk_async(snapshot);
                });
            }
        });

        // 2.2b 切换历史会话
        let w_weak_switch = w_weak.clone();
        ai_bridge.on_switch_session(move |id| {
            if let Some(w) = w_weak_switch.upgrade() {
                DRAWER_STATE.with(|state_cell| {
                    let mut state = state_cell.borrow_mut();
                    if let Some(arch) = state.history_archives.iter().find(|a| a.id == id.as_str()).cloned() {
                        let h_id = if arch.host_id.is_empty() { "default".to_string() } else { arch.host_id.clone() };
                        let mut restored = HostAiSession::new(&h_id, &arch.host_name);
                        restored.messages = arch.messages.clone();
                        state.ai_sessions.insert(h_id.clone(), restored);
                        sync_ai_for_host_inner(&w, &mut state, &h_id, &arch.host_name);
                        let snapshot = snapshot_stored_ai_data(&state);
                        save_ai_data_to_disk_async(snapshot);
                    }
                });
            }
        });

        // 2.2c 删除历史会话
        let w_weak_del = w_weak.clone();
        ai_bridge.on_delete_session(move |id| {
            if let Some(w) = w_weak_del.upgrade() {
                DRAWER_STATE.with(|state_cell| {
                    let mut state = state_cell.borrow_mut();
                    state.history_archives.retain(|a| a.id != id.as_str());
                    let history_ui_list: Vec<AiHistorySession> = state.history_archives.iter()
                        .map(|a| AiHistorySession {
                            id: a.id.to_shared(),
                            title: a.title.to_shared(),
                            message_count: a.messages.len() as i32,
                            updated_time: a.updated_time.to_shared(),
                        })
                        .collect();
                    w.global::<AiBridge>().set_history_sessions(to_model_rc(history_ui_list));
                    let snapshot = snapshot_stored_ai_data(&state);
                    save_ai_data_to_disk_async(snapshot);
                });
            }
        });

        // 2.2d 模型变更
        let w_weak_model = w_weak.clone();
        ai_bridge.on_model_selected(move |m| {
            if let Some(w) = w_weak_model.upgrade() {
                DRAWER_STATE.with(|state_cell| {
                    let mut state = state_cell.borrow_mut();
                    let h_id = state.current_host_id.clone();
                    if let Some(session) = state.ai_sessions.get_mut(&h_id) {
                        session.selected_model = m.to_string();
                    }
                });
                w.global::<AiBridge>().set_selected_model(m);
            }
        });

        // 2.2e 思考模式变更
        let w_weak_mode = w_weak.clone();
        ai_bridge.on_mode_selected(move |m| {
            if let Some(w) = w_weak_mode.upgrade() {
                DRAWER_STATE.with(|state_cell| {
                    let mut state = state_cell.borrow_mut();
                    let h_id = state.current_host_id.clone();
                    if let Some(session) = state.ai_sessions.get_mut(&h_id) {
                        session.thinking_degree = m.to_string();
                    }
                });
                w.global::<AiBridge>().set_thinking_degree(m);
            }
        });

        // 2.2e2 安全审核策略变更 (单层 5 档：纯人工 / 只读巡检 / 辅助运维 / 免审直达 / AI自审)
        let w_weak_policy = w_weak.clone();
        ai_bridge.on_audit_policy_selected(move |policy| {
            if let Some(w) = w_weak_policy.upgrade() {
                let pol_str = policy.to_string();
                DRAWER_STATE.with(|state_cell| {
                    let mut state = state_cell.borrow_mut();
                    let h_id = state.current_host_id.clone();
                    if let Some(session) = state.ai_sessions.get_mut(&h_id) {
                        session.audit_policy = pol_str.clone();
                        match pol_str.as_str() {
                            "纯人工" => { session.audit_mode = "人工".into(); }
                            "只读巡检" => { session.audit_mode = "自动".into(); session.auto_audit_level = "安全".into(); }
                            "辅助运维" => { session.audit_mode = "自动".into(); session.auto_audit_level = "警告".into(); }
                            "免审直达" => { session.audit_mode = "自动".into(); session.auto_audit_level = "危险".into(); }
                            "AI自审" => { session.audit_mode = "AI".into(); }
                            _ => {}
                        }
                    }
                });
                w.global::<AiBridge>().set_audit_policy(policy.clone());
                match pol_str.as_str() {
                    "纯人工" => { w.global::<AiBridge>().set_audit_mode("人工".into()); }
                    "只读巡检" => {
                        w.global::<AiBridge>().set_audit_mode("自动".into());
                        w.global::<AiBridge>().set_auto_audit_level("安全".into());
                    }
                    "辅助运维" => {
                        w.global::<AiBridge>().set_audit_mode("自动".into());
                        w.global::<AiBridge>().set_auto_audit_level("警告".into());
                    }
                    "免审直达" => {
                        w.global::<AiBridge>().set_audit_mode("自动".into());
                        w.global::<AiBridge>().set_auto_audit_level("危险".into());
                    }
                    "AI自审" => { w.global::<AiBridge>().set_audit_mode("AI".into()); }
                    _ => {}
                }
            }
        });

        // 2.2f 审核方式变更 (人工 / 自动 / AI - 兼容老调用)
        let w_weak_audit = w_weak.clone();
        ai_bridge.on_audit_selected(move |a| {
            if let Some(w) = w_weak_audit.upgrade() {
                let a_str = a.to_string();
                DRAWER_STATE.with(|state_cell| {
                    let mut state = state_cell.borrow_mut();
                    let h_id = state.current_host_id.clone();
                    if let Some(session) = state.ai_sessions.get_mut(&h_id) {
                        session.audit_mode = a_str.clone();
                    }
                });
                w.global::<AiBridge>().set_audit_mode(a);
            }
        });

        // 2.2g 自动审核子级别变更 (拒绝 / 安全 / 警告 / 危险)
        let w_weak_auto_lvl = w_weak.clone();
        ai_bridge.on_auto_audit_level_selected(move |lvl| {
            if let Some(w) = w_weak_auto_lvl.upgrade() {
                let lvl_str = lvl.to_string();
                DRAWER_STATE.with(|state_cell| {
                    let mut state = state_cell.borrow_mut();
                    let h_id = state.current_host_id.clone();
                    if let Some(session) = state.ai_sessions.get_mut(&h_id) {
                        session.auto_audit_level = lvl_str.clone();
                    }
                });
                w.global::<AiBridge>().set_auto_audit_level(lvl.clone());
                // 🛑 双向联动：同步更新设置页面的审核安全级别！
                w.global::<SettingsBridge>().set_setting_ai_auto_audit_level(lvl);
            }
        });

        // 2.3 填入终端命令行
        let w_weak_apply = w_weak.clone();
        ai_bridge.on_apply_suggested_cmd(move |cmd| {
            if let Some(w) = w_weak_apply.upgrade() {
                w.global::<TerminalBridge>().invoke_send_snippet(cmd);
            }
        });

        // 2.4 立即注入并回车执行
        let w_weak_inject = w_weak.clone();
        ai_bridge.on_inject_and_run_cmd(move |cmd| {
            if let Some(w) = w_weak_inject.upgrade() {
                let cmd_with_nl = format!("{}\n", cmd);
                w.global::<TerminalBridge>().invoke_send_snippet(cmd_with_nl.into());
            }
        });

        // 2.5 复制推荐指令到系统剪贴板
        ai_bridge.on_copy_command(move |cmd| {
            let cmd_str = cmd.to_string();
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(&cmd_str);
            }
            DRAWER_STATE.with(|state_cell| {
                let state = state_cell.borrow();
                if let Some(ref c) = state.ctx {
                    c.notifications.success("复制成功", "命令已复制到剪贴板");
                }
            });
        });

        // 2.6 更新消息的审核决策状态 (二选一单次决策: approved / rejected)
        let w_weak_audit_update = w_weak.clone();
        ai_bridge.on_update_message_audit(move |msg_id, status| {
            if let Some(w) = w_weak_audit_update.upgrade() {
                let id_str = msg_id.to_string();
                let status_str = status.to_string();
                DRAWER_STATE.with(|state_cell| {
                    let mut state = state_cell.borrow_mut();
                    let h_id = state.current_host_id.clone();
                    if let Some(session) = state.ai_sessions.get_mut(&h_id) {
                        for m in session.messages.iter_mut() {
                            if m.id == id_str.as_str() {
                                m.audit_status = status_str.to_shared();
                                break;
                            }
                        }
                    }
                    sync_ai_for_host_inner(&w, &mut state, &h_id, "当前终端");
                    let snapshot = snapshot_stored_ai_data(&state);
                    save_ai_data_to_disk_async(snapshot);
                });
            }
        });

        // 2.7 切换思考过程展开/折叠
        let w_weak_thinking = w_weak.clone();
        ai_bridge.on_toggle_thinking_expanded(move |msg_id| {
            if let Some(w) = w_weak_thinking.upgrade() {
                let id_str = msg_id.to_string();
                DRAWER_STATE.with(|state_cell| {
                    let mut state = state_cell.borrow_mut();
                    let h_id = state.current_host_id.clone();
                    if let Some(session) = state.ai_sessions.get_mut(&h_id) {
                        for m in session.messages.iter_mut() {
                            if m.id == id_str.as_str() {
                                m.is_thinking_expanded = !m.is_thinking_expanded;
                                break;
                            }
                        }
                    }
                    sync_ai_for_host_inner(&w, &mut state, &h_id, "当前终端");
                });
            }
        });
    }

    // -------------------------------------------------------------------------
    // 3. 挂载 MonitorBridge 刷新指标回调
    // -------------------------------------------------------------------------
    {
        let w_weak = window.as_weak();
        window.global::<MonitorBridge>().on_refresh_metrics(move || {
            if let Some(w) = w_weak.upgrade() {
                DRAWER_STATE.with(|state_cell| {
                    let mut state = state_cell.borrow_mut();
                    let h_id = state.current_host_id.clone();
                    let h_name = w.global::<TerminalBridge>().get_active_host_name().to_string();
                    state.sampling_phase = state.sampling_phase.wrapping_add(1);
                    let metrics = compute_sample_metrics(&h_name, &h_id, state.sampling_phase);
                    w.global::<MonitorBridge>().set_metrics(metrics);
                });
            }
        });
    }

    // -------------------------------------------------------------------------
    // 4. 挂载 TerminalBridge tmux 管理回调
    // -------------------------------------------------------------------------
    {
        let w_weak = window.as_weak();
        window.global::<TerminalBridge>().on_refresh_tmux(move || {
            if let Some(w) = w_weak.upgrade() {
                let tb = w.global::<TerminalBridge>();
                let h_id = tb.get_active_host_id().to_string();
                let h_name = tb.get_active_host_name().to_string();
                sync_tmux_for_host(&w, &h_id, &h_name);
            }
        });
    }

    {
        let w_weak = window.as_weak();
        window.global::<TerminalBridge>().on_install_tmux(move |cmd| {
            if let Some(w) = w_weak.upgrade() {
                let cmd_with_nl = format!("{}\n", cmd);
                w.global::<TerminalBridge>().invoke_send_snippet(cmd_with_nl.into());
            }
        });
    }

    {
        let w_weak = window.as_weak();
        window.global::<TerminalBridge>().on_kill_tmux_session(move |name| {
            if let Some(w) = w_weak.upgrade() {
                let session_name = name.to_string();
                let tb = w.global::<TerminalBridge>();
                let h_id = tb.get_active_host_id().to_string();
                let h_name = tb.get_active_host_name().to_string();

                // 1. 乐观更新：立即从当前 UI 列表中移除该会话 (0ms 极速反馈)
                let remaining: Vec<TmuxSessionItem> = (0..tb.get_tmux_sessions().row_count())
                    .filter_map(|i| tb.get_tmux_sessions().row_data(i))
                    .filter(|s| s.name.as_str() != session_name.as_str())
                    .collect();
                crate::store::diff::update_model_rc_in_place(
                    &tb.get_tmux_sessions(),
                    remaining,
                    |m| tb.set_tmux_sessions(m),
                );

                // 2. 后台异步执行真实 kill 命令 (不污染终端用户输入)
                let is_local = h_id == "local" || h_id.starts_with("local-");
                let safe_name = sanitize_tmux_name(&session_name);
                let w_weak_task = w.as_weak();
                let launch_cfg_opt = if !is_local {
                    DRAWER_STATE.with(|state_cell| {
                        state_cell.try_borrow().ok().and_then(|s| {
                            s.ctx.as_ref().and_then(|c| crate::handlers::file_handlers::resolve_host_launch_config(c, &h_id))
                        })
                    }).or_else(|| {
                        crate::handlers::file_handlers::with_file_app_ctx(|c| {
                            crate::handlers::file_handlers::resolve_host_launch_config(c, &h_id)
                        }).flatten()
                    })
                } else {
                    None
                };

                crate::async_util::spawn_async(async move {
                    if is_local {
                        let _ = tokio::process::Command::new("tmux")
                            .args(["kill-session", "-t", &safe_name])
                            .output()
                            .await;
                    } else if let Some(cfg) = launch_cfg_opt {
                        let kill_cmd = format!("tmux kill-session -t '{}' 2>/dev/null || true", safe_name);
                        let _ = smagical_ssh::execute_remote(&cfg, &kill_cmd).await;
                    }

                    // 执行完成后触发一次静默核对刷新
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w_ui) = w_weak_task.upgrade() {
                            sync_tmux_for_host(&w_ui, &h_id, &h_name);
                        }
                    });
                });
            }
        });
    }

    {
        let w_weak = window.as_weak();
        window.global::<TerminalBridge>().on_create_tmux_session(move |name| {
            if let Some(w) = w_weak.upgrade() {
                let session_name = name.to_string();
                let tb = w.global::<TerminalBridge>();
                let h_id = tb.get_active_host_id().to_string();
                let h_name = tb.get_active_host_name().to_string();
                tb.set_tmux_is_loading(true);

                let is_local = h_id == "local" || h_id.starts_with("local-");
                let safe_name = sanitize_tmux_name(&session_name);
                let w_weak_task = w.as_weak();
                let launch_cfg_opt = if !is_local {
                    DRAWER_STATE.with(|state_cell| {
                        state_cell.try_borrow().ok().and_then(|s| {
                            s.ctx.as_ref().and_then(|c| crate::handlers::file_handlers::resolve_host_launch_config(c, &h_id))
                        })
                    }).or_else(|| {
                        crate::handlers::file_handlers::with_file_app_ctx(|c| {
                            crate::handlers::file_handlers::resolve_host_launch_config(c, &h_id)
                        }).flatten()
                    })
                } else {
                    None
                };

                crate::async_util::spawn_async(async move {
                    if is_local {
                        let _ = tokio::process::Command::new("tmux")
                            .args(["new-session", "-d", "-s", &safe_name])
                            .output()
                            .await;
                    } else if let Some(cfg) = launch_cfg_opt {
                        let new_cmd = format!("tmux new-session -d -s '{}' 2>/dev/null || true", safe_name);
                        let _ = smagical_ssh::execute_remote(&cfg, &new_cmd).await;
                    }

                    // 创建完成后立即触发一次刷新
                    let _ = slint::invoke_from_event_loop(move || {
                        if let Some(w_ui) = w_weak_task.upgrade() {
                            sync_tmux_for_host(&w_ui, &h_id, &h_name);
                        }
                    });
                });
            }
        });
    }
}

/// 当终端焦点或激活会话发生切换时，同步右侧栏各工具状态与每机隔离会话
pub(crate) fn sync_right_drawers_on_session_change(
    window: &AppWindow,
    new_host_id: &str,
    new_host_name: &str,
) {
    DRAWER_STATE.with(|state_cell| {
        let mut state = state_cell.borrow_mut();
        let old_host_id = state.current_host_id.clone();

        // 1. 如果此前有聚焦主机，且输入框有草稿，在切换前保存至旧主机会话中
        if !old_host_id.is_empty() && old_host_id != new_host_id {
            let draft = window.global::<AiBridge>().get_input_text().to_string();
            if let Some(sess) = state.ai_sessions.get_mut(&old_host_id) {
                sess.draft_input = draft;
            }
        }

        // 同主机多标签切换守卫：如果切换前后宿主主机相同，无需重置伴生抽屉与模型
        if !new_host_id.is_empty() && old_host_id == new_host_id {
            return;
        }

        state.current_host_id = new_host_id.to_string();

        let wb = window.global::<WindowBridge>();
        let is_open = wb.get_is_right_drawer_open();
        let active_tool = wb.get_active_right_tool().to_string();

        if new_host_id.is_empty() {
            // 所有终端关闭：右侧栏统一重置为优雅空状态
            stop_monitor_sampling_inner(window, &mut state);
            stop_tmux_polling_inner(window, &mut state);
            let mb = window.global::<MonitorBridge>();
            mb.set_has_active_host(false);
            mb.set_is_sampling(false);

            let ai_b = window.global::<AiBridge>();
            ai_b.set_has_active_host(false);
            ai_b.set_active_host_name("".into());
            ai_b.set_active_host_id("".into());
            ai_b.set_messages(ModelRc::default());
            ai_b.set_input_text("".into());
            ai_b.set_is_generating(false);

            let tb = window.global::<TunnelsBridge>();
            tb.set_is_local_terminal(false);
            tb.set_active_host_name("未连接主机".into());
            tb.set_active_host_id("".into());
            tb.set_host_tunnels(ModelRc::default());

            let term_b = window.global::<TerminalBridge>();
            term_b.set_tmux_sessions(ModelRc::default());
            term_b.set_tmux_is_loading(false);
            term_b.set_tmux_is_installed(true);
            return;
        }

        // 当前有活跃终端主机：确保 MonitorBridge 与 TunnelsBridge 状态就绪
        window.global::<MonitorBridge>().set_has_active_host(true);

        let is_local = new_host_id.starts_with("local-") || new_host_id == "local";
        let tb = window.global::<TunnelsBridge>();
        tb.set_is_local_terminal(is_local);
        if is_local {
            tb.set_active_host_name(if new_host_name.is_empty() { "本地终端".into() } else { new_host_name.into() });
            tb.set_active_host_id(new_host_id.into());
            tb.set_host_tunnels(ModelRc::default());
        }

        // 2. 无论抽屉是否展开，同步 AI 会话上下文与草稿
        sync_ai_for_host_inner(window, &mut state, new_host_id, new_host_name);

        // 3. 抽屉展开时，实时同步当前激活工具数据
        if is_open {
            if active_tool == "tunnel" {
                stop_tmux_polling_inner(window, &mut state);
                if let Some(ref c) = state.ctx {
                    crate::handlers::tunnel_handlers::sync_ui_host_tunnels(window, c);
                }
            } else if active_tool == "monitor" {
                stop_tmux_polling_inner(window, &mut state);
                start_monitor_sampling_inner(
                    window,
                    &mut state,
                    new_host_id,
                    new_host_name,
                );
            } else if active_tool == "sftp" {
                stop_tmux_polling_inner(window, &mut state);
                if let Some(ref c) = state.ctx {
                    crate::handlers::file_handlers::sync_sftp_drawer_for_host(window, c, new_host_id, new_host_name);
                }
            } else if active_tool == "tmux" {
                start_tmux_polling_inner(window, &mut state, new_host_id, new_host_name);
            } else {
                stop_tmux_polling_inner(window, &mut state);
            }
        } else {
            stop_tmux_polling_inner(window, &mut state);
        }
    });
}

/// 当某个主机的终端会话关闭时，触发伴生资源生命周期治理
pub(crate) fn handle_host_session_closed(
    window: &AppWindow,
    closed_host_id: &str,
    remaining_host_tabs: usize,
) {
    if closed_host_id.is_empty() {
        return;
    }

    if remaining_host_tabs == 0 {
        tracing::info!(
            target: "smagical_ui::companion",
            "[宿主级联释放] 主机 [{}] 终端已彻底销毁，开始排空并释放伴生资源...",
            closed_host_id
        );

        DRAWER_STATE.with(|state_cell| {
            let mut state = state_cell.borrow_mut();

            // 1. AI: 智能熔断后台 API 并释放内存会话
            if let Some(sess) = state.ai_sessions.remove(closed_host_id) {
                sess.abort_flag.store(true, Ordering::SeqCst);
                // 检查是否有用户对话，如果有，自动归档保存，防止用户对话数据丢失
                let user_msgs: Vec<&AiChatMessage> = sess.messages.iter()
                    .filter(|m| m.sender == "user")
                    .collect();
                if !user_msgs.is_empty() {
                    let first_prompt = user_msgs[0].content.to_string();
                    let title = if first_prompt.chars().count() > 16 {
                        format!("{}...", first_prompt.chars().take(16).collect::<String>())
                    } else {
                        first_prompt
                    };
                    let time_str = {
                        let now = std::time::SystemTime::now();
                        let s = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                        let h = ((s % 86400) / 3600 + 8) % 24;
                        let m = (s % 3600) / 60;
                        format!("{:02}:{:02}", h, m)
                    };
                    let archive = ArchivedAiSession {
                        id: format!("arch-{}", uuid::Uuid::new_v4().simple()),
                        title,
                        host_id: closed_host_id.to_string(),
                        host_name: sess.host_name.clone(),
                        messages: sess.messages,
                        updated_time: time_str,
                    };
                    state.history_archives.insert(0, archive);
                    let history_ui_list: Vec<AiHistorySession> = state.history_archives.iter()
                        .map(|a| AiHistorySession {
                            id: a.id.to_shared(),
                            title: a.title.to_shared(),
                            message_count: a.messages.len() as i32,
                            updated_time: a.updated_time.to_shared(),
                        })
                        .collect();
                    window.global::<AiBridge>().set_history_sessions(to_model_rc(history_ui_list));
                }
                let snapshot = snapshot_stored_ai_data(&state);
                save_ai_data_to_disk_async(snapshot);
                tracing::info!(
                    target: "smagical_ui::companion",
                    "[AI会话释放与持久化] 已保存并释放主机 [{}] 会话",
                    closed_host_id
                );
            }

            // 2. 若当前无激活终端，确保监控停止并重置 UI 状态
            let term_b = window.global::<TerminalBridge>();
            let has_active = term_b.get_has_active_session();
            if !has_active {
                stop_monitor_sampling_inner(window, &mut state);
                stop_tmux_polling_inner(window, &mut state);
                window.global::<MonitorBridge>().set_has_active_host(false);
                window.global::<AiBridge>().set_has_active_host(false);
                window.global::<AiBridge>().set_active_host_name("".into());
                window.global::<AiBridge>().set_active_host_id("".into());
                window.global::<AiBridge>().set_messages(ModelRc::default());
                window.global::<TunnelsBridge>().set_host_tunnels(ModelRc::default());
                term_b.set_tmux_sessions(ModelRc::default());
                term_b.set_tmux_is_loading(false);
                term_b.set_tmux_is_installed(true);
                state.current_host_id = String::new();
            }
        });
    }
}

/// 应用退出前清理伴生资源
#[allow(dead_code)]
pub(crate) fn cleanup_on_exit() {
    DRAWER_STATE.with(|state_cell| {
        let mut state = state_cell.borrow_mut();
        if let Some(t) = state.monitor_timer.take() {
            t.stop();
        }
        if let Some(t) = state.tmux_timer.take() {
            t.stop();
        }
        for s in state.ai_sessions.values() {
            s.abort_flag.store(true, Ordering::SeqCst);
        }
        let snapshot = snapshot_stored_ai_data(&state);
        save_ai_data_to_disk_sync(&snapshot);
    });
}

/// 当从设置页面修改全局安全审核级别时，同步当前活动主机的 AI session 与 UI
pub(crate) fn update_current_host_auto_audit_level(window: &AppWindow, audit_level: &str) {
    DRAWER_STATE.with(|state_cell| {
        let mut state = state_cell.borrow_mut();
        let h_id = state.current_host_id.clone();
        if let Some(session) = state.ai_sessions.get_mut(&h_id) {
            session.auto_audit_level = audit_level.to_string();
        }
    });
    window.global::<AiBridge>().set_auto_audit_level(audit_level.into());
}

#[allow(dead_code)]
pub(crate) fn update_current_host_audit_mode(window: &AppWindow, audit_mode: &str) {
    DRAWER_STATE.with(|state_cell| {
        let mut state = state_cell.borrow_mut();
        let h_id = state.current_host_id.clone();
        if let Some(session) = state.ai_sessions.get_mut(&h_id) {
            session.audit_mode = audit_mode.to_string();
        }
    });
    window.global::<AiBridge>().set_audit_mode(audit_mode.into());
}

/// 同步指定主机的独立 AI 会话至 UI 视图
fn sync_ai_for_host_inner(
    window: &AppWindow,
    state: &mut RightDrawerState,
    host_id: &str,
    host_name: &str,
) {
    let ai_b = window.global::<AiBridge>();
    if host_id.is_empty() {
        ai_b.set_active_host_id("".into());
        ai_b.set_active_host_name("".into());
        ai_b.set_has_active_host(false);
        ai_b.set_messages(ModelRc::default());
        ai_b.set_input_text("".into());
        ai_b.set_is_generating(false);
        return;
    }

    ai_b.set_active_host_id(host_id.into());
    ai_b.set_active_host_name(host_name.into());
    ai_b.set_has_active_host(true);

    let session = state.ai_sessions
        .entry(host_id.to_string())
        .or_insert_with(|| {
            let mut s = HostAiSession::new(host_id, host_name);
            let global_audit = window.global::<SettingsBridge>().get_setting_ai_auto_audit_level().to_string();
            if !global_audit.is_empty() {
                s.auto_audit_level = global_audit;
            }
            s
        });

    crate::store::diff::update_model_rc_in_place(
        &ai_b.get_messages(),
        session.messages.clone(),
        |m| ai_b.set_messages(m),
    );
    ai_b.set_input_text(session.draft_input.to_shared());
    ai_b.set_is_generating(session.is_generating);
    ai_b.set_selected_model(session.selected_model.to_shared());
    ai_b.set_thinking_degree(session.thinking_degree.to_shared());
    ai_b.set_audit_policy(session.audit_policy.to_shared());
    ai_b.set_audit_mode(session.audit_mode.to_shared());
    ai_b.set_auto_audit_level(session.auto_audit_level.to_shared());
}

/// 启动针对指定主机的实时监控探针定时采样器 (1秒/次)
fn start_monitor_sampling_inner(
    window: &AppWindow,
    state: &mut RightDrawerState,
    host_id: &str,
    host_name: &str,
) {
    let mb = window.global::<MonitorBridge>();
    if host_id.is_empty() {
        mb.set_has_active_host(false);
        mb.set_is_sampling(false);
        stop_monitor_sampling_inner(window, state);
        return;
    }

    mb.set_has_active_host(true);
    mb.set_is_sampling(true);

    // 先立即渲染一帧当前主机初始指标
    state.sampling_phase = state.sampling_phase.wrapping_add(1);
    let initial_metrics = compute_sample_metrics(host_name, host_id, state.sampling_phase);
    mb.set_metrics(initial_metrics);

    // 建立 1500ms 周期性探针采样定时器
    let w_weak = window.as_weak();
    let h_id = host_id.to_string();
    let h_name = host_name.to_string();
    let sampler = Arc::clone(&state.linux_sampler);
    let is_busy = Arc::clone(&state.is_sampling_busy);

    let launch_cfg_opt = state.ctx.as_ref().and_then(|c| {
        crate::handlers::file_handlers::resolve_host_launch_config(c, &h_id)
    });

    let metrics_service = state.ctx.as_ref().map(|c| c.core_state.metrics());

    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(1500), move || {
        if let Some(w) = w_weak.upgrade() {
            if let Some(ref cfg) = launch_cfg_opt {
                // 远程主机模式：通过后台异步任务非阻塞采样真实 Linux 内核指标
                if !is_busy.swap(true, Ordering::SeqCst) {
                    let sampler_clone = Arc::clone(&sampler);
                    let is_busy_clone = Arc::clone(&is_busy);
                    let w_weak_task = w.as_weak();
                    let h_id_task = h_id.clone();
                    let h_name_task = h_name.clone();
                    let cfg_clone = cfg.clone();
                    let metrics_svc_clone = metrics_service.clone();

                    crate::async_util::spawn_async(async move {
                        let mut real_metrics_opt = None;

                        // 1. 优先通过 CoreState 契约驱动 HostMetricsService 执行非阻塞系统指标采集
                        if let Some(ref svc) = metrics_svc_clone {
                            if let Ok(snapshot) = svc.sample_metrics(&h_id_task).await {
                                let mut slint_data = compute_sample_metrics(&h_name_task, &h_id_task, 0);
                                slint_data.cpu_usage = (snapshot.cpu_usage_percent / 100.0).clamp(0.01, 1.0);
                                if snapshot.memory_total_bytes > 0 {
                                    slint_data.ram_usage = (snapshot.memory_used_bytes as f32 / snapshot.memory_total_bytes as f32).clamp(0.01, 1.0);
                                    slint_data.ram_used = smagical_ssh::monitor::format_bytes(snapshot.memory_used_bytes).into();
                                    slint_data.ram_total = smagical_ssh::monitor::format_bytes(snapshot.memory_total_bytes).into();
                                }
                                if snapshot.disk_total_bytes > 0 {
                                    slint_data.disk_usage = (snapshot.disk_used_bytes as f32 / snapshot.disk_total_bytes as f32).clamp(0.01, 1.0);
                                }
                                slint_data.net_rx_rate = smagical_ssh::monitor::format_rate(snapshot.rx_bytes_per_sec as f32 / (1024.0 * 1024.0)).into();
                                slint_data.net_tx_rate = smagical_ssh::monitor::format_rate(snapshot.tx_bytes_per_sec as f32 / (1024.0 * 1024.0)).into();
                                slint_data.last_update_time = "刚刚 (服务契约)".into();
                                real_metrics_opt = Some(slint_data);
                            }
                        }

                        // 2. 若未就绪，回退至远程全量采样解析
                        if real_metrics_opt.is_none() {
                            let host_addr = format!("{}:{}", cfg_clone.host, cfg_clone.port);
                            if let Ok(real_metrics) = sampler_clone.sample_remote(
                                cfg_clone,
                                h_name_task.clone(),
                                host_addr,
                            ).await {
                                real_metrics_opt = Some(real_metrics.into_slint());
                            }
                        }

                        is_busy_clone.store(false, Ordering::SeqCst);

                        if let Some(metrics_to_render) = real_metrics_opt {
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(w_ui) = w_weak_task.upgrade() {
                                    DRAWER_STATE.with(|state_cell| {
                                        let state = state_cell.borrow();
                                        if state.current_host_id == h_id_task {
                                            w_ui.global::<MonitorBridge>().set_metrics(metrics_to_render);
                                        }
                                    });
                                }
                            });
                        }
                    });
                }
            } else {
                // 本地终端模式：平滑时序回退
                DRAWER_STATE.with(|state_cell| {
                    let mut state = state_cell.borrow_mut();
                    if state.current_host_id != h_id {
                        return;
                    }
                    state.sampling_phase = state.sampling_phase.wrapping_add(1);
                    let m = compute_sample_metrics(&h_name, &h_id, state.sampling_phase);
                    w.global::<MonitorBridge>().set_metrics(m);
                });
            }
        }
    });

    state.monitor_timer = Some(timer);
}

/// 停止实时监控探针采样 (关抽屉或切到无会话时调用)
fn stop_monitor_sampling_inner(window: &AppWindow, state: &mut RightDrawerState) {
    if let Some(timer) = state.monitor_timer.take() {
        timer.stop();
    }
    window.global::<MonitorBridge>().set_is_sampling(false);
}

/// 启动针对指定主机的实时 tmux 会话轮询 (2秒/次，静默增量刷新)
fn start_tmux_polling_inner(
    window: &AppWindow,
    state: &mut RightDrawerState,
    host_id: &str,
    host_name: &str,
) {
    stop_tmux_polling_inner(window, state);

    if host_id.is_empty() {
        return;
    }

    // 首次进入：触发一次显式带 loading 状态的快速探活拉取
    sync_tmux_for_host_with_ctx(window, state.ctx.as_ref(), host_id, host_name, true);

    // 启动 2s 周期性静默轮询 (show_loading = false，无闪烁/无卡顿)
    let w_weak = window.as_weak();
    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(2000),
        move || {
            if let Some(w) = w_weak.upgrade() {
                let wb = w.global::<WindowBridge>();
                if !wb.get_is_right_drawer_open() || wb.get_active_right_tool() != "tmux" {
                    return;
                }

                let (h_id, ctx_opt) = match DRAWER_STATE.with(|state_cell| {
                    state_cell
                        .try_borrow()
                        .ok()
                        .map(|s| (s.current_host_id.clone(), s.ctx.clone()))
                }) {
                    Some(pair) => pair,
                    None => return,
                };

                if h_id.is_empty() {
                    return;
                }

                sync_tmux_for_host_with_ctx(&w, ctx_opt.as_ref(), &h_id, "", false);
            }
        },
    );

    state.tmux_timer = Some(timer);
}

/// 停止 tmux 轮询定时器 (关抽屉或切到其它工具/会话时调用)
fn stop_tmux_polling_inner(_window: &AppWindow, state: &mut RightDrawerState) {
    if let Some(timer) = state.tmux_timer.take() {
        timer.stop();
    }
}

/// 同步并刷新目标主机的 tmux 会话列表及探活状态 (手动触发或入口调用，默认显示 loading)
pub(crate) fn sync_tmux_for_host(window: &AppWindow, host_id: &str, host_name: &str) {
    let ctx_opt = DRAWER_STATE.with(|state_cell| {
        state_cell.try_borrow().ok().and_then(|s| s.ctx.clone())
    });
    sync_tmux_for_host_with_ctx(window, ctx_opt.as_ref(), host_id, host_name, true);
}

/// 支持直接传入已有 AppContext 引用的内部同步实现，完全规避 DRAWER_STATE RefCell 重入借用冲突
pub(crate) fn sync_tmux_for_host_with_ctx(
    window: &AppWindow,
    ctx: Option<&AppContext>,
    host_id: &str,
    _host_name: &str,
    show_loading: bool,
) {
    let term_b = window.global::<TerminalBridge>();
    if host_id.is_empty() {
        term_b.set_tmux_is_loading(false);
        term_b.set_tmux_is_installed(false);
        term_b.set_tmux_sessions(ModelRc::default());
        return;
    }

    // 原子防重入标志：若上一次异步 SSH 探活/会话列表拉取尚未返回，跳过当前轮询
    let is_busy = DRAWER_STATE.with(|state_cell| {
        state_cell.try_borrow().ok().map(|s| s.is_tmux_busy.clone())
    })
    .unwrap_or_else(|| Arc::new(AtomicBool::new(false)));

    if is_busy
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    if show_loading {
        term_b.set_tmux_is_loading(true);
    }

    let h_id = host_id.to_string();
    let is_local = h_id == "local" || h_id.starts_with("local-");

    let launch_cfg_opt = if !is_local {
        ctx.and_then(|c| crate::handlers::file_handlers::resolve_host_launch_config(c, &h_id))
            .or_else(|| {
                crate::handlers::file_handlers::with_file_app_ctx(|c| {
                    crate::handlers::file_handlers::resolve_host_launch_config(c, &h_id)
                })
                .flatten()
            })
    } else {
        None
    };

    let w_weak = window.as_weak();
    let is_busy_clone = is_busy.clone();
    let h_id_task = h_id.clone();

    crate::async_util::spawn_async(async move {
        let probe_result = if is_local {
            // 本地探活与查询
            let check_cmd = if cfg!(target_os = "windows") {
                "where.exe"
            } else {
                "which"
            };
            let installed = tokio::process::Command::new(check_cmd)
                .arg("tmux")
                .output()
                .await
                .map(|o| o.status.success())
                .unwrap_or(false);

            if !installed {
                Some((false, Vec::new()))
            } else {
                let list_out = tokio::process::Command::new("tmux")
                    .args([
                        "list-sessions",
                        "-F",
                        "#{session_name}|#{session_windows}|#{session_attached}|#{session_created}",
                    ])
                    .output()
                    .await;
                let items = match list_out {
                    Ok(out) if out.status.success() => {
                        let text = String::from_utf8_lossy(&out.stdout);
                        parse_tmux_session_output(&text)
                    }
                    _ => Vec::new(),
                };
                Some((true, items))
            }
        } else if let Some(cfg) = launch_cfg_opt {
            // 远程单行复合指令：原子探活 + 获取会话列表 (单次 SSH 往返，极速响应)
            let probe_cmd = "if command -v tmux >/dev/null 2>&1; then echo 'TMUX_OK'; tmux list-sessions -F '#{session_name}|#{session_windows}|#{session_attached}|#{session_created}' 2>/dev/null || true; else echo 'TMUX_NONE'; fi";
            let check_res = smagical_ssh::execute_remote(&cfg, probe_cmd).await;
            match check_res {
                Ok(out) if out.status.success() => {
                    let text = String::from_utf8_lossy(&out.stdout);
                    let trimmed = text.trim();
                    if trimmed.starts_with("TMUX_NONE") {
                        Some((false, Vec::new()))
                    } else if trimmed.starts_with("TMUX_OK") {
                        let items = parse_tmux_session_output(&text);
                        Some((true, items))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        } else {
            Some((false, Vec::new()))
        };

        is_busy_clone.store(false, Ordering::SeqCst);

        let _ = slint::invoke_from_event_loop(move || {
            if let Some(w) = w_weak.upgrade() {
                let tb = w.global::<TerminalBridge>();
                tb.set_tmux_is_loading(false);
                if let Some((is_installed, sessions)) = probe_result {
                    DRAWER_STATE.with(|state_cell| {
                        if let Ok(state) = state_cell.try_borrow() {
                            if state.current_host_id != h_id_task {
                                return;
                            }
                            tb.set_tmux_is_installed(is_installed);
                            crate::store::diff::update_model_rc_in_place(
                                &tb.get_tmux_sessions(),
                                sessions,
                                |m| tb.set_tmux_sessions(m),
                            );
                        }
                    });
                }
            }
        });
    });
}

/// 安全过滤 tmux 会话名，杜绝非法注入字符并确保 tmux 命名规范
fn sanitize_tmux_name(name: &str) -> String {
    let clean: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if clean.is_empty() {
        "session".to_string()
    } else {
        clean
    }
}

/// 解析 tmux list-sessions 输出为 UI 条目列表 (支持复合指令与回退兼容)
fn parse_tmux_session_output(output: &str) -> Vec<TmuxSessionItem> {
    let mut sessions = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() || line == "TMUX_OK" || line == "TMUX_NONE" {
            continue;
        }
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 4 {
            let name = parts[0].trim();
            let windows_count: i32 = parts[1].trim().parse().unwrap_or(1);
            let is_attached = parts[2].trim() != "0";
            let created_ts: u64 = parts[3].trim().parse().unwrap_or(0);

            let created_time = if created_ts > 0 {
                chrono::DateTime::from_timestamp(created_ts as i64, 0)
                    .map(|dt| dt.format("%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "未知".to_string())
            } else {
                "未知".to_string()
            };

            sessions.push(TmuxSessionItem {
                id: name.into(),
                name: name.into(),
                windows_count,
                is_attached,
                created_time: created_time.into(),
            });
        } else if let Some((name_part, rest)) = line.split_once(':') {
            // 兼容未格式化的原生回退输出: "0: 1 windows (created Wed Oct 8) [80x24] (attached)"
            let name = name_part.trim();
            let is_attached = rest.contains("(attached)");
            let windows_count: i32 = rest
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1);
            sessions.push(TmuxSessionItem {
                id: name.into(),
                name: name.into(),
                windows_count,
                is_attached,
                created_time: "刚刚".into(),
            });
        }
    }
    sessions
}

/// 生成平滑动态时序指标与波形
fn compute_sample_metrics(host_name: &str, host_ip: &str, phase: usize) -> SystemMetricsData {
    let sin_val = ((phase as f32) * 0.25).sin();
    let cos_val = ((phase as f32) * 0.18).cos();

    let cpu_pct_val = (0.28 + sin_val * 0.12).clamp(0.05, 0.95);
    let ram_pct_val = (0.62 + cos_val * 0.08).clamp(0.10, 0.92);

    let cpu_user = format!("{:.1}%", cpu_pct_val * 70.0);
    let cpu_sys = format!("{:.1}%", cpu_pct_val * 30.0);

    let net_rx_mb = 8.5 + sin_val * 4.2;
    let net_tx_mb = 2.4 + cos_val * 1.8;

    // 动态生成横跨 300px 宽度的 30 秒时序平滑折线与闭合面积 (X=0..=300，共 31 采样点，每秒向左平移 10px)
    let build_sparkline = |base_fn: &dyn Fn(f32) -> f32| -> (String, String) {
        let mut line = String::with_capacity(320);
        let mut area = String::with_capacity(380);
        let mut first_y = 30.0;

        for i in 0..=30 {
            let x = i * 10;
            let t_f = (phase as f32) + (i as f32) - 30.0;
            let y = base_fn(t_f).clamp(6.0, 44.0);

            if i == 0 {
                first_y = y;
                line.push_str(&format!("M 0 {:.1}", y));
            } else {
                line.push_str(&format!(" L {} {:.1}", x, y));
            }
        }

        area.push_str(&format!("M 0 48 L 0 {:.1}", first_y));
        for i in 1..=30 {
            let x = i * 10;
            let t_f = (phase as f32) + (i as f32) - 30.0;
            let y = base_fn(t_f).clamp(6.0, 44.0);
            area.push_str(&format!(" L {} {:.1}", x, y));
        }
        area.push_str(" L 300 48 Z");

        (line, area)
    };

    // 1. CPU 负载 30s 波形
    let (cpu_line, cpu_area) = build_sparkline(&|t| {
        let s1 = (t * 0.22).sin();
        let c1 = (t * 0.15).cos();
        let s2 = (t * 0.45).sin();
        let ratio = (0.35 + s1 * 0.18 + c1 * 0.10 + s2 * 0.05).clamp(0.08, 0.92);
        42.0 - ratio * 34.0
    });

    // 2. 网络下行 (RX) 30s 波形 (绿色)
    let (net_rx_line, net_rx_area) = build_sparkline(&|t| {
        let s = (t * 0.20).sin();
        let c = (t * 0.35).cos();
        let ratio = (0.42 + s * 0.24 + c * 0.14).clamp(0.05, 0.95);
        43.0 - ratio * 35.0
    });

    // 3. 网络上行 (TX) 30s 波形 (紫色)
    let (net_tx_line, net_tx_area) = build_sparkline(&|t| {
        let s = (t * 0.18 + 1.2).sin();
        let c = (t * 0.26).cos();
        let ratio = (0.26 + s * 0.18 + c * 0.08).clamp(0.05, 0.88);
        44.0 - ratio * 34.0
    });

    let display_h_name = if host_name.is_empty() { "prod-server" } else { host_name };
    let display_h_ip = if host_ip.is_empty() { "192.168.1.100:22" } else { host_ip };

    SystemMetricsData {
        host_name: display_h_name.into(),
        host_ip: display_h_ip.into(),
        os_name: "Linux (Kernel 6.8)".into(),
        virt_type: "KVM 容器云".into(),
        uptime: "18天 07时 34分".into(),
        load_avg: format!("{:.2}, {:.2}, {:.2}", cpu_pct_val * 1.5, cpu_pct_val * 1.2, cpu_pct_val * 1.0).into(),
        fd_usage: "2,410 / 104万 (0.2%)".into(),
        tasks_threads: "4 活跃 / 256 线程".into(),
        active_users: 1,
        cpu_usage: cpu_pct_val,
        cpu_cores: 8,
        cpu_freq: "3.20 GHz".into(),
        cpu_temp: format!("{}℃", 46 + (phase % 5)).into(),
        cpu_user_pct: cpu_user.into(),
        cpu_sys_pct: cpu_sys.into(),
        cpu_iowait_pct: "0.1%".into(),
        cpu_steal_pct: "0.0%".into(),
        ram_usage: ram_pct_val,
        ram_used: format!("{:.1} GB", ram_pct_val * 16.0).into(),
        ram_total: "16.0 GB".into(),
        ram_available: format!("{:.1} GB", (1.0 - ram_pct_val) * 16.0).into(),
        ram_cached: "3.2 GB".into(),
        ram_free: "2.1 GB".into(),
        swap_enabled: true,
        swap_usage: 0.12,
        swap_used: "480 MB".into(),
        swap_total: "4.0 GB".into(),
        swap_free: "3.5 GB".into(),
        swap_status_text: "健康 (无频发换入)".into(),
        net_interface: "eth0".into(),
        net_rx_rate: format!("{:.1} MB/s", net_rx_mb.max(0.1)).into(),
        net_tx_rate: format!("{:.1} MB/s", net_tx_mb.max(0.1)).into(),
        net_total_rx: "142.8 GB".into(),
        net_total_tx: "48.2 GB".into(),
        tcp_inuse: 24,
        tcp_tw: 12,
        udp_inuse: 8,
        net_drops: 0,
        net_errs: 0,
        ping_ms: 16,
        probe_latency_ms: 12,
        last_update_time: "刚刚".into(),
        poll_interval: "1s".into(),
        cpu_sparkline_area: cpu_area.into(),
        cpu_sparkline_line: cpu_line.into(),
        net_rx_sparkline_area: net_rx_area.into(),
        net_rx_sparkline_line: net_rx_line.into(),
        net_tx_sparkline_area: net_tx_area.into(),
        net_tx_sparkline_line: net_tx_line.into(),
        memory_usage: ram_pct_val,
        disk_usage: 0.38,
        uptime_days: 18,
        process_count: 152,
    }
}

/// 智能生成针对特定主机的运维 Prompt 与排查指令建议 (支持根据模型、模式、审核级别随机生成富交互内容)
fn generate_ai_devops_response(
    host_name: &str,
    user_query: &str,
    model: &str,
    mode: &str,
    audit_mode: &str,
    auto_audit_level: &str,
) -> (String, String, String, String, String, String) {
    struct CmdCandidate {
        category: &'static str,
        cmd: &'static str,
        risk: &'static str,
        desc: &'static str,
        explanation: &'static str,
    }

    let candidates = [
        // 磁盘与文件诊断
        CmdCandidate {
            category: "disk",
            cmd: "du -ahx / 2>/dev/null | sort -rh | head -n 10",
            risk: "low",
            desc: "磁盘大文件排查",
            explanation: "按实际物理占用只读扫描前 10 个体积最大的目录与文件，安全无写操作。",
        },
        CmdCandidate {
            category: "disk",
            cmd: "find /var/log -type f -size +50M -exec ls -lh {} + 2>/dev/null",
            risk: "low",
            desc: "日志大文件定位",
            explanation: "迅速定位 /var/log 下超过 50MB 的过期堆积日志，方便进一步轮转与清理。",
        },
        CmdCandidate {
            category: "disk",
            cmd: "df -hT / && lsblk -f",
            risk: "low",
            desc: "分区与文件系统类型检查",
            explanation: "输出全盘挂载点、可用空间百分比以及底层块设备 UUID 与文件系统格式。",
        },

        // 内存与进程
        CmdCandidate {
            category: "memory",
            cmd: "ps -eo pid,user,%cpu,%mem,command --sort=-%mem | head -n 10",
            risk: "low",
            desc: "内存消耗前10进程排行榜",
            explanation: "只读采集 RSS 常驻内存集占比最高的服务进程，辅助定位潜在内存泄漏。",
        },
        CmdCandidate {
            category: "memory",
            cmd: "cat /proc/meminfo | grep -E \"MemTotal|MemAvailable|Buffers|Cached|Swap\"",
            risk: "low",
            desc: "物理内存与Swap健康度分析",
            explanation: "解析 Linux 内核核心内存分配计数器，评估可用内存水位与缓存回收空间。",
        },
        CmdCandidate {
            category: "memory",
            cmd: "vmstat 1 5",
            risk: "low",
            desc: "虚拟内存时序采样",
            explanation: "按 1 秒间隔采样 5 次进程队列、换入换出(si/so)及中断上下文切换频率。",
        },

        // CPU 与系统负载
        CmdCandidate {
            category: "cpu",
            cmd: "top -b -n 1 | head -n 20",
            risk: "low",
            desc: "CPU 综合健康快照",
            explanation: "批处理模式输出当前瞬时负载、运行队列与 CPU 密集型任务排行。",
        },
        CmdCandidate {
            category: "cpu",
            cmd: "uptime && cat /proc/loadavg",
            risk: "low",
            desc: "系统负载与运行时间查询",
            explanation: "展示系统 1、5、15 分钟指数平均负载与当前活跃进程核数比值。",
        },

        // 容器与集群
        CmdCandidate {
            category: "docker",
            cmd: "docker ps -a --format \"table {{.Names}}\t{{.Status}}\t{{.Ports}}\"",
            risk: "low",
            desc: "容器集群全量状态概览",
            explanation: "快速列出当前宿主机所有容器生命周期状态与外部端口映射映射表。",
        },
        CmdCandidate {
            category: "docker",
            cmd: "docker stats --no-stream --format \"table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.NetIO}}\"",
            risk: "low",
            desc: "活跃容器资源占用水位",
            explanation: "非阻塞只读抓取当前在线容器的 CPU/内存及网络吞吐指标快照。",
        },

        // 网络与端口
        CmdCandidate {
            category: "network",
            cmd: "ss -tulpn | grep LISTEN",
            risk: "low",
            desc: "监听端口与绑定服务排查",
            explanation: "替代传统 netstat，快速输出所有 TCP/UDP 正在监听的 Socket 与所属进程 PID。",
        },
        CmdCandidate {
            category: "network",
            cmd: "ip -br a && ip route show",
            risk: "low",
            desc: "网卡 IP 与路由表诊断",
            explanation: "精简模式展示物理网卡与虚拟网桥链路状态、IPv4/IPv6 配置及默认网关路由。",
        },
        CmdCandidate {
            category: "network",
            cmd: "curl -Iv -m 5 https://google.com 2>&1 | head -n 25",
            risk: "low",
            desc: "外部网络连通性与握手探测",
            explanation: "验证出站 DNS 解析、TCP 握手延时及 TLS/SSL 证书链有效性。",
        },

        // 日志与故障排查
        CmdCandidate {
            category: "log",
            cmd: "journalctl -p 3 -xb --no-pager -n 50",
            risk: "low",
            desc: "系统级严重错误(Error/Crit)过滤",
            explanation: "从 systemd 日志缓冲池中过滤当前启动周期内的核心告警与异常崩溃追踪。",
        },
        CmdCandidate {
            category: "log",
            cmd: "dmesg -T --level=err,warn | tail -n 30",
            risk: "low",
            desc: "内核环形缓冲区告警排查",
            explanation: "带时间戳展示硬件驱动、OOM-Killer 触发及文件系统只读保护等内核级事件。",
        },

        // 高危与服务变更 (可用于演示高危拦截审核流程)
        CmdCandidate {
            category: "restart",
            cmd: "systemctl restart nginx && systemctl status nginx --no-pager",
            risk: "high",
            desc: "Web 服务平滑重启与状态检查",
            explanation: "重启主 Nginx 守护进程并回显执行状态，可能引起瞬间网络连接断开。",
        },
        CmdCandidate {
            category: "clean",
            cmd: "sync; echo 3 > /proc/sys/vm/drop_caches",
            risk: "high",
            desc: "强制落盘并释放页面缓存/目录项",
            explanation: "清空 Linux 页缓存 (PageCache) 与 Slab 对象，属于高危系统级写操作。",
        },
    ];

    let q = user_query.trim().to_lowercase();
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as usize;

    let matched: Vec<&CmdCandidate> = if q.contains("磁盘") || q.contains("存储") || q.contains("df") || q.contains("du") {
        candidates.iter().filter(|c| c.category == "disk").collect()
    } else if q.contains("内存") || q.contains("free") || q.contains("oom") || q.contains("swap") {
        candidates.iter().filter(|c| c.category == "memory").collect()
    } else if q.contains("cpu") || q.contains("负载") || q.contains("top") {
        candidates.iter().filter(|c| c.category == "cpu").collect()
    } else if q.contains("docker") || q.contains("容器") || q.contains("k8s") {
        candidates.iter().filter(|c| c.category == "docker").collect()
    } else if q.contains("网络") || q.contains("端口") || q.contains("ip") || q.contains("ping") {
        candidates.iter().filter(|c| c.category == "network").collect()
    } else if q.contains("日志") || q.contains("log") || q.contains("error") || q.contains("故障") {
        candidates.iter().filter(|c| c.category == "log").collect()
    } else if q.contains("重启") || q.contains("kill") || q.contains("stop") || q.contains("清理") {
        candidates.iter().filter(|c| c.category == "restart" || c.category == "clean").collect()
    } else {
        Vec::new()
    };

    let chosen = if !matched.is_empty() {
        matched[seed % matched.len()]
    } else {
        &candidates[seed % candidates.len()]
    };

    // 审核状态判定策略 (支持单层 5 档扁平策略，并与上下文语义研判深度协同)
    let (audit_status, audit_tip) = match audit_mode {
        "纯人工" | "人工" => (
            "pending".to_string(),
            "【人工核准】当前处于纯人工确认模式，所有生成指令均需操作者手动核准执行".to_string(),
        ),
        "只读巡检" => {
            if chosen.risk == "low" {
                (
                    "executed".to_string(),
                    "【只读放行】检测为安全只读探测指令，已自动打入终端执行".to_string(),
                )
            } else {
                (
                    "pending".to_string(),
                    "【只读拦截】检测为系统配置与数据变更指令，在【只读巡检】级别下已挂起等待人工确认".to_string(),
                )
            }
        }
        "辅助运维" => {
            if chosen.risk == "high" {
                (
                    "pending".to_string(),
                    "【高危拦截】检测为破坏性高危操作指令，安全基线已强行挂起等待人工授权".to_string(),
                )
            } else {
                (
                    "executed".to_string(),
                    "【辅助放行】检测为常规运维启停或修改指令，已自动打入终端执行".to_string(),
                )
            }
        }
        "免审直达" => (
            "executed".to_string(),
            "【免审直达】零拦截免审模式已生效，全量指令已自动打入终端执行".to_string(),
        ),
        "AI自审" | "AI" => {
            if chosen.risk == "high" {
                (
                    "pending".to_string(),
                    format!("【AI自主研判拦截】大模型感知当前主机 [{}] 上下文，判定此命令存在高破坏风险，已主动挂起并请示确认", host_name),
                )
            } else {
                (
                    "executed".to_string(),
                    format!("【AI自主裁决放行】大模型评估该操作在主机 [{}] 的当前上下文中处于安全可控范围，已自动打入终端执行", host_name),
                )
            }
        }
        _ => {
            match auto_audit_level {
                "拒绝" => (
                    "pending".to_string(),
                    "【拒绝拦截】自动策略已拒绝全部操作，等待操作者手动核准".to_string(),
                ),
                "安全" => {
                    if chosen.risk == "low" {
                        (
                            "executed".to_string(),
                            "【安全放行】检测为安全只读指令，已自动打入终端执行".to_string(),
                        )
                    } else {
                        (
                            "pending".to_string(),
                            "【安全拦截】检测为配置变更指令，已挂起等待人工审批".to_string(),
                        )
                    }
                }
                "警告" => {
                    if chosen.risk == "high" {
                        (
                            "pending".to_string(),
                            "【高危拦截】检测为高危破坏性操作，已拦截挂起".to_string(),
                        )
                    } else {
                        (
                            "executed".to_string(),
                            "【常规放行】检测为常规操作/只读指令，已自动打入终端执行".to_string(),
                        )
                    }
                }
                "危险" => (
                    "executed".to_string(),
                    "【完全放行】危险模式已激活，全量指令免审已自动打入终端执行".to_string(),
                ),
                _ => {
                    if chosen.risk == "low" {
                        ("executed".to_string(), "【安全放行】只读指令已自动打入终端执行".to_string())
                    } else {
                        ("pending".to_string(), "【安全拦截】已挂起等待审批".to_string())
                    }
                }
            }
        }
    };

    let thinking_text = if mode.contains("深度思考") || mode.contains("CoT") {
        format!(
            "🧠 深度思考链 (CoT) · 模型: {}\n\
            1. 意图解析: 解析用户指令「{}」，目标主机: [{}]\n\
            2. 上下文推演: 识别目标场景为「{}」，匹配生产级运维模式\n\
            3. 安全评级: 评估为「{} 风险」，安全准则生效\n\
            4. 审核模式: [{}] (策略级别: {}) ➔ {}\n\
            5. 命令合成: 采用只读/幂等保护机制，构建输出结果。",
            model,
            if user_query.is_empty() { "日常巡检" } else { user_query },
            host_name,
            chosen.desc,
            chosen.risk,
            audit_mode,
            auto_audit_level,
            audit_tip
        )
    } else if mode.contains("常规") || mode.contains("Fast") {
        format!(
            "⚡ 快速推理模式 · 模型: {}\n\
            结合主机 [{}] 系统基准，完成意图「{}」识别。\n\
            匹配策略: {} (安全级别: {}, 审核模式: {})，审核状态: {}。",
            model, host_name, chosen.desc, chosen.explanation, chosen.risk, audit_mode, audit_tip
        )
    } else {
        format!(
            "🚀 极速响应 · 模型: {}\n快速匹配针对 [{}] 的「{}」推荐指令。",
            model, host_name, chosen.desc
        )
    };

    let reply_text = format!(
        "根据当前上下文分析，已为您在主机 [{}] 上定制生成「{}」排查建议：\n{}\n\n推荐执行以下运维指令：",
        host_name, chosen.desc, chosen.explanation
    );

    (chosen.cmd.to_string(), chosen.risk.to_string(), audit_status, audit_tip, thinking_text, reply_text)
}
