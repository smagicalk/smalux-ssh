//! 右侧伴生工具栏抽屉生命周期治理、惰性加载、终端焦点同步与每机独立会话服务。
//!
//! 核心设计规范：
//! 1. 宿主从属生命周期 (Host-Scoped Cascading)：右侧栏完全服务于当前终端；终端销毁时伴生资源释放（FollowApp 隧道例外）。
//! 2. 惰性加载与关即停 (Lazy & Ephemeral)：抽屉打开时按需启动探针；抽屉关闭或最小化时唯独实时监控停止采样，其余后台长任务继续。
//! 3. 独立 AI 会话隔离 (Per-Host AI Isolation)：每台主机拥有完全独立的上下文会话流与草稿，互不串线，切终端无缝热置换。
//! 4. 主线程零竞态治理 (Main-Thread Direct Dispatch)：规避跨线程锁竞争与 Send 限制，与 Slint UI 运行循环同频。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use slint::{ComponentHandle, ModelRc, VecModel};

use crate::generated::{
    AiBridge, AiChatMessage, AiHistorySession, AppWindow, MonitorBridge, SettingsBridge, SystemMetricsData, TerminalBridge,
    TunnelsBridge, WindowBridge,
};
use crate::handlers::AppContext;

/// 历史会话归档实体
#[derive(Clone)]
pub(crate) struct ArchivedAiSession {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) host_id: String,
    pub(crate) host_name: String,
    pub(crate) messages: Vec<AiChatMessage>,
    pub(crate) updated_time: String,
}

/// 单台主机独立的 AI 运维伴生会话实体
#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct HostAiSession {
    pub(crate) host_id: String,
    pub(crate) host_name: String,
    pub(crate) messages: Vec<AiChatMessage>,
    pub(crate) draft_input: String,
    pub(crate) selected_model: String,
    pub(crate) thinking_degree: String,
    pub(crate) audit_policy: String,
    pub(crate) audit_mode: String,
    pub(crate) auto_audit_level: String,
    pub(crate) is_generating: bool,
    pub(crate) abort_flag: Arc<AtomicBool>,
}

impl HostAiSession {
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

/// 右侧伴生抽屉主线程运行时状态中枢
pub(crate) struct RightDrawerState {
    pub(crate) ctx: Option<AppContext>,
    pub(crate) ai_sessions: HashMap<String, HostAiSession>,
    pub(crate) history_archives: Vec<ArchivedAiSession>,
    pub(crate) monitor_timer: Option<slint::Timer>,
    pub(crate) sampling_phase: usize,
    pub(crate) current_host_id: String,
}

impl Default for RightDrawerState {
    fn default() -> Self {
        Self {
            ctx: None,
            ai_sessions: HashMap::new(),
            history_archives: Vec::new(),
            monitor_timer: None,
            sampling_phase: 0,
            current_host_id: String::new(),
        }
    }
}

thread_local! {
    static DRAWER_STATE: RefCell<RightDrawerState> = RefCell::new(RightDrawerState::default());
}

/// 注册右侧伴生工具栏全套生命周期与 UI 回调处理器
pub(crate) fn register_right_drawer_handlers(window: &AppWindow, ctx: &AppContext) {
    DRAWER_STATE.with(|state_cell| {
        state_cell.borrow_mut().ctx = Some(ctx.clone());
    });

    // -------------------------------------------------------------------------
    // 1. 挂载 WindowBridge 抽屉切换与开闭回调
    // -------------------------------------------------------------------------
    {
        let w_weak = window.as_weak();
        window.global::<WindowBridge>().on_switch_right_tool(move |tool_id| {
            if let Some(w) = w_weak.upgrade() {
                let wb = w.global::<WindowBridge>();
                wb.set_active_right_tool(tool_id.clone());
                wb.set_is_right_drawer_open(true);

                DRAWER_STATE.with(|state_cell| {
                    let mut state = state_cell.borrow_mut();
                    if let Some(ref c) = state.ctx {
                        c.core_state.toggle_right_panel(&tool_id);
                    }

                    let term_b = w.global::<TerminalBridge>();
                    let h_id = if !state.current_host_id.is_empty() {
                        state.current_host_id.clone()
                    } else {
                        term_b.get_active_host_id().to_string()
                    };
                    let h_name = term_b.get_active_host_name().to_string();
                    state.current_host_id = h_id.clone();

                    if tool_id == "monitor" {
                        start_monitor_sampling_inner(&w, &mut state, &h_id, &h_name);
                    } else {
                        stop_monitor_sampling_inner(&w, &mut state);
                        if tool_id == "tunnel" {
                            if let Some(ref c) = state.ctx {
                                crate::handlers::tunnel_handlers::sync_ui_host_tunnels(&w, c);
                            }
                        } else if tool_id == "ai" {
                            sync_ai_for_host_inner(&w, &mut state, &h_id, &h_name);
                        } else if tool_id == "sftp" {
                            if let Some(ref c) = state.ctx {
                                crate::handlers::file_handlers::sync_sftp_drawer_for_host(&w, c, &h_id, &h_name);
                            }
                        }
                    }
                });
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
                    let mut state = state_cell.borrow_mut();
                    if let Some(ref c) = state.ctx {
                        c.core_state.right_panels().write().unwrap().set_drawer_open(is_open);
                    }

                    let term_b = w.global::<TerminalBridge>();
                    let h_id = if !state.current_host_id.is_empty() {
                        state.current_host_id.clone()
                    } else {
                        term_b.get_active_host_id().to_string()
                    };
                    let h_name = term_b.get_active_host_name().to_string();
                    state.current_host_id = h_id.clone();

                    if !is_open {
                        // 🛑 唯独监控探针在抽屉关闭时立即休眠！其余后台长任务（隧道、传输、AI）保持常驻
                        stop_monitor_sampling_inner(&w, &mut state);
                    } else {
                        if tool_id == "monitor" {
                            start_monitor_sampling_inner(&w, &mut state, &h_id, &h_name);
                        } else {
                            stop_monitor_sampling_inner(&w, &mut state);
                            if tool_id == "tunnel" {
                                if let Some(ref c) = state.ctx {
                                    crate::handlers::tunnel_handlers::sync_ui_host_tunnels(&w, c);
                                }
                            } else if tool_id == "ai" {
                                sync_ai_for_host_inner(&w, &mut state, &h_id, &h_name);
                            } else if tool_id == "sftp" {
                                if let Some(ref c) = state.ctx {
                                    crate::handlers::file_handlers::sync_sftp_drawer_for_host(&w, c, &h_id, &h_name);
                                }
                            }
                        }
                    }
                });
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

                let (h_id, h_name, auto_audit_level, abort_flag) = DRAWER_STATE.with(|state_cell| {
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

                    let session = state.ai_sessions.entry(h_id.clone()).or_insert_with(|| HostAiSession::new(&h_id, &h_name));
                    session.messages.push(user_msg);
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
                    (h_id, h_name, auto_audit_level, Some(abort_flag))
                });

                if let Some(abort_flag) = abort_flag {
                    w.global::<AiBridge>().set_input_text("".into());

                    // 异步模拟大模型推理 (1.4 秒后返回针对该主机的建议与指令)
                    let w_weak_timer = w.as_weak();
                    let content_str = content.to_string();
                    let h_id_timer = h_id.clone();
                    let h_name_timer = h_name.clone();
                    let m_str = model_str.clone();
                    let t_str = thinking_str.clone();
                    let a_str = audit_str.clone();
                    let auto_lvl_str = auto_audit_level.clone();

                    slint::Timer::single_shot(std::time::Duration::from_millis(1400), move || {
                        if abort_flag.load(Ordering::SeqCst) {
                            tracing::info!(target: "smagical_ui::ai", "AI 生成任务已被终端销毁熔断取消");
                            return;
                        }

                        let (suggested_cmd, risk_level, audit_status, audit_reason, thinking_text, reply_text) =
                            generate_ai_devops_response(&h_name_timer, &content_str, &m_str, &t_str, &a_str, &auto_lvl_str);

                        let is_executed = audit_status == "executed";
                        let cmd_to_run = suggested_cmd.clone();

                        let ai_reply = AiChatMessage {
                            id: format!("ai-{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis()).into(),
                            sender: "assistant".into(),
                            content: reply_text.into(),
                            thinking_content: thinking_text.into(),
                            is_thinking_expanded: true,
                            suggested_cmd: suggested_cmd.into(),
                            cmd_risk_level: risk_level.into(),
                            audit_status: audit_status.into(),
                            audit_reason: audit_reason.into(),
                            timestamp: "刚刚".into(),
                        };

                        DRAWER_STATE.with(|state_cell| {
                            let mut state = state_cell.borrow_mut();
                            if let Some(session) = state.ai_sessions.get_mut(&h_id_timer) {
                                session.messages.push(ai_reply);
                                session.is_generating = false;
                            }

                            // 如果用户当前聚焦在该主机或未绑定具体主机，刷新 UI；若已切去别机，保存在缓冲区
                            if let Some(w2) = w_weak_timer.upgrade() {
                                if state.current_host_id == h_id_timer
                                    || state.current_host_id.is_empty()
                                    || h_id_timer == "default"
                                    || state.current_host_id == "default"
                                {
                                    sync_ai_for_host_inner(&w2, &mut state, &h_id_timer, &h_name_timer);
                                    // 自动执行模式下，直接注入当前活动终端并回车执行
                                    if is_executed && !cmd_to_run.is_empty() {
                                        let cmd_with_nl = format!("{}\n", cmd_to_run);
                                        w2.global::<TerminalBridge>().invoke_send_snippet(cmd_with_nl.into());
                                    }
                                }
                            }
                        });
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
                            id: a.id.clone().into(),
                            title: a.title.clone().into(),
                            message_count: a.messages.len() as i32,
                            updated_time: a.updated_time.clone().into(),
                        })
                        .collect();
                    w.global::<AiBridge>().set_history_sessions(ModelRc::from(Rc::new(VecModel::from(history_ui_list))));

                    // 3. 重置当前主机/全局会话为全新会话
                    state.ai_sessions.insert(target_id.clone(), HostAiSession::new(&target_id, &h_name));
                    sync_ai_for_host_inner(&w, &mut state, &target_id, &h_name);
                    w.global::<AiBridge>().set_input_text("".into());
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
                            id: a.id.clone().into(),
                            title: a.title.clone().into(),
                            message_count: a.messages.len() as i32,
                            updated_time: a.updated_time.clone().into(),
                        })
                        .collect();
                    w.global::<AiBridge>().set_history_sessions(ModelRc::from(Rc::new(VecModel::from(history_ui_list))));
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
                                m.audit_status = status_str.clone().into();
                                break;
                            }
                        }
                    }
                    sync_ai_for_host_inner(&w, &mut state, &h_id, "当前终端");
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

        state.current_host_id = new_host_id.to_string();

        let wb = window.global::<WindowBridge>();
        let is_open = wb.get_is_right_drawer_open();
        let active_tool = wb.get_active_right_tool().to_string();

        if new_host_id.is_empty() {
            // 所有终端关闭：右侧栏统一重置为优雅空状态
            stop_monitor_sampling_inner(window, &mut state);
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
                if let Some(ref c) = state.ctx {
                    crate::handlers::tunnel_handlers::sync_ui_host_tunnels(window, c);
                }
            } else if active_tool == "monitor" {
                start_monitor_sampling_inner(
                    window,
                    &mut state,
                    new_host_id,
                    new_host_name,
                );
            } else if active_tool == "sftp" {
                if let Some(ref c) = state.ctx {
                    crate::handlers::file_handlers::sync_sftp_drawer_for_host(window, c, new_host_id, new_host_name);
                }
            }
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
                tracing::info!(
                    target: "smagical_ui::companion",
                    "[AI会话释放] 已中止主机 [{}] 未完成的推理并清除内存会话",
                    closed_host_id
                );
            }

            // 2. 若当前无激活终端，确保监控停止并重置 UI 状态
            let term_b = window.global::<TerminalBridge>();
            let has_active = term_b.get_has_active_session();
            if !has_active {
                stop_monitor_sampling_inner(window, &mut state);
                window.global::<MonitorBridge>().set_has_active_host(false);
                window.global::<AiBridge>().set_has_active_host(false);
                window.global::<AiBridge>().set_active_host_name("".into());
                window.global::<AiBridge>().set_active_host_id("".into());
                window.global::<AiBridge>().set_messages(ModelRc::default());
                window.global::<TunnelsBridge>().set_host_tunnels(ModelRc::default());
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
        for s in state.ai_sessions.values() {
            s.abort_flag.store(true, Ordering::SeqCst);
        }
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

    let model = ModelRc::from(Rc::new(VecModel::from(session.messages.clone())));
    ai_b.set_messages(model);
    ai_b.set_input_text(session.draft_input.clone().into());
    ai_b.set_is_generating(session.is_generating);
    ai_b.set_selected_model(session.selected_model.clone().into());
    ai_b.set_thinking_degree(session.thinking_degree.clone().into());
    ai_b.set_audit_policy(session.audit_policy.clone().into());
    ai_b.set_audit_mode(session.audit_mode.clone().into());
    ai_b.set_auto_audit_level(session.auto_audit_level.clone().into());
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

    // 建立 1000ms 周期性探针采样定时器
    let w_weak = window.as_weak();
    let h_id = host_id.to_string();
    let h_name = host_name.to_string();

    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(1000), move || {
        if let Some(w) = w_weak.upgrade() {
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
