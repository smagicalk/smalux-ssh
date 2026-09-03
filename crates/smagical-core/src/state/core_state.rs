use std::sync::{Arc, RwLock};
use crate::domain::{
    ActiveTerminalSessionContext, ActivityBarRegistry, NavigationRequest, NavigationRouter,
    RightPanelRegistry, TerminalAction,
};
use crate::event::{
    ConfigChangedEvent, CredentialDeletedEvent, CredentialSavedEvent,
    CredentialSecretCopiedEvent, EventDispatcher, EventManager, FileOperationBeforeEvent,
    KeyGeneratedEvent, ModuleActivatedEvent, ModuleDeactivatedEvent,
    NavigationRequestedEvent, PasswordGeneratedEvent, RightPanelRegisteredEvent,
    RightPanelSwitchedEvent, RightPanelUnregisteredEvent, TerminalActionRequestedEvent,
    TerminalFocusChangedEvent, TerminalSessionEvent, ThemeChangedEvent,
    WindowStateChangedEvent,
};
use crate::storage::{AppStorage, MockStorage};

/// 无界面依赖的核心状态引擎 (Core State Engine)
///
/// 内部封装聚合存储门面 (`Arc<dyn AppStorage>`)、集中式事件分发管理器 (`Arc<EventManager>`)、
/// 侧边栏动态注册中心 (`Arc<ActivityBarRegistry>`)、右侧辅助面板注册中心 (`Arc<RwLock<RightPanelRegistry>>`)、
/// 当前活跃终端上下文 (`Arc<RwLock<Option<ActiveTerminalSessionContext>>>`) 以及全局统一导航路由中枢 (`Arc<RwLock<NavigationRouter>>`)。
#[derive(Clone)]
pub struct CoreState {
    storage: Arc<RwLock<Arc<dyn AppStorage>>>,
    is_mock: Arc<RwLock<bool>>,
    event_manager: Arc<EventManager>,
    activity_bar: Arc<ActivityBarRegistry>,
    right_panels: Arc<RwLock<RightPanelRegistry>>,
    active_terminal: Arc<RwLock<Option<ActiveTerminalSessionContext>>>,
    navigation: Arc<RwLock<NavigationRouter>>,
}

fn attach_default_event_loggers(events: &EventManager) {
    let g1 = events.global().listen(|e: &CredentialSavedEvent| {
        tracing::info!(
            target: "smalux::credential",
            "[事件总线:凭据保存] ID: [{}], 名称: '{}', 算法: '{}', 类型: {:?}, 指纹: {:?}, 是否新建: {}",
            e.cred_id, e.name, e.algorithm, e.cred_type, e.fingerprint, e.is_new
        );
    });
    g1.detach();

    let g2 = events.global().listen(|e: &CredentialDeletedEvent| {
        tracing::warn!(
            target: "smalux::credential",
            "[事件总线:凭据删除] ID: [{}] 已从本地保管库中安全清除",
            e.cred_id
        );
    });
    g2.detach();

    let g3 = events.global().listen(|e: &CredentialSecretCopiedEvent| {
        if e.is_sensitive {
            tracing::warn!(
                target: "smalux::security",
                "[安全审计:机密提取] 凭据 ID: [{}], 名称: '{}', 动作: 复制【{:?}】至系统剪贴板 (高危安全操作)",
                e.cred_id, e.name, e.copy_type
            );
        } else {
            tracing::info!(
                target: "smalux::credential",
                "[事件总线:机密复制] 凭据 ID: [{}], 复制类型: {:?}",
                e.cred_id, e.copy_type
            );
        }
    });
    g3.detach();

    let g4 = events.global().listen(|e: &KeyGeneratedEvent| {
        tracing::info!(
            target: "smalux::credential",
            "[事件总线:密钥生成] 规格: '{}', 生成公钥指纹: [{}]",
            e.algorithm, e.fingerprint
        );
    });
    g4.detach();

    let g5 = events.global().listen(|_: &PasswordGeneratedEvent| {
        tracing::debug!(
            target: "smalux::credential",
            "[事件总线:密码生成] 已生成全新的高强度随机密码"
        );
    });
    g5.detach();

    let g6 = events.global().listen(|e: &FileOperationBeforeEvent| {
        if e.action == "delete" && !e.is_remote {
            let p = e.path.to_lowercase();
            if p == "/" || p == "c:\\" || p == "c:/" || p.contains("windows\\system32") || p.contains("/etc/passwd") {
                tracing::warn!(target: "smalux::security", "[安全守护] 拦截高危文件删除尝试: {}", e.path);
                e.abort("禁止删除操作系统关键核心文件或根目录！");
            }
        }
    });
    g6.detach();

    let g7 = events.global().listen(|e: &TerminalSessionEvent| {
        tracing::info!(
            target: "smalux::terminal",
            "[事件总线:终端会话] 会话 ID: [{}], 主机: [{}], 动作: {}",
            e.session_id, e.host_id, e.action
        );
    });
    g7.detach();

    let g8 = events.global().listen(|e: &ConfigChangedEvent| {
        tracing::info!(
            target: "smalux::config",
            "[事件总线:配置变动] 键: '{}', 旧值: '{}', 新值: '{}', 来源: '{}'",
            e.key, e.old_val, e.new_val, e.source
        );
    });
    g8.detach();

    let g9 = events.global().listen(|e: &ThemeChangedEvent| {
        tracing::info!(
            target: "smalux::theme",
            "[事件总线:主题切换] 主题 ID: '{}', 深色模式: {}",
            e.theme_id, e.is_dark
        );
    });
    g9.detach();

    let g10 = events.global().listen(|e: &WindowStateChangedEvent| {
        tracing::info!(
            target: "smalux::window",
            "[事件总线:窗口状态] 状态变更: {}",
            e.state
        );
    });
    g10.detach();

    let g11 = events.global().listen(|e: &crate::event::SnippetSavedEvent| {
        tracing::info!(
            target: "smalux::snippet",
            "[事件总线:代码片段保存] ID: [{}], 标题: '{}', 是否新建: {}",
            e.snippet_id, e.title, e.is_new
        );
    });
    g11.detach();

    let g12 = events.global().listen(|e: &crate::event::SnippetDeletedEvent| {
        tracing::warn!(
            target: "smalux::snippet",
            "[事件总线:代码片段删除] ID: [{}] 已从代码片段库中移除",
            e.snippet_id
        );
    });
    g12.detach();

    let g13 = events.global().listen(|e: &crate::event::SnippetExecutedEvent| {
        tracing::info!(
            target: "smalux::snippet",
            "[事件总线:代码片段执行] ID: [{}], 目标终端: {:?}, 自动执行: {}",
            e.snippet_id, e.session_id, e.auto_execute
        );
    });
    g13.detach();

    let g14 = events.global().listen(|e: &crate::event::SnippetGroupSavedEvent| {
        tracing::info!(
            target: "smalux::snippet",
            "[事件总线:代码片段分组保存] ID: [{}], 名称: '{}', 父分组: {:?}, 是否新建: {}",
            e.group_id, e.name, e.parent_id, e.is_new
        );
    });
    g14.detach();

    let g15 = events.global().listen(|e: &crate::event::SnippetGroupDeletedEvent| {
        tracing::warn!(
            target: "smalux::snippet",
            "[事件总线:代码片段分组删除] ID: [{}] 分组已被彻底删除",
            e.group_id
        );
    });
    g15.detach();

    let g16 = events.global().listen(|e: &crate::event::TunnelSavedEvent| {
        tracing::info!(
            target: "smalux::tunnel",
            "[事件总线:网络隧道保存] ID: [{}], 名称: '{}', 类型: '{}', 是否新建: {}",
            e.tunnel_id, e.name, e.tunnel_type, e.is_new
        );
    });
    g16.detach();

    let g17 = events.global().listen(|e: &crate::event::TunnelDeletedEvent| {
        tracing::warn!(
            target: "smalux::tunnel",
            "[事件总线:网络隧道删除] ID: [{}] 已从网络配置库中移除",
            e.tunnel_id
        );
    });
    g17.detach();

    let g18 = events.global().listen(|e: &crate::event::TunnelStateChangedEvent| {
        if e.is_running {
            tracing::info!(
                target: "smalux::tunnel",
                "[事件总线:网络隧道启动] ID: [{}] 隧道已成功启动建立连接",
                e.tunnel_id
            );
        } else {
            tracing::info!(
                target: "smalux::tunnel",
                "[事件总线:网络隧道停止] ID: [{}] 隧道已关闭",
                e.tunnel_id
            );
        }
    });
    g18.detach();

    let g19 = events.global().listen(|e: &crate::event::TunnelBeforeSaveEvent| {
        // 1. 跳板机成环与循环依赖检测
        let mut seen = std::collections::HashSet::new();
        for h_id in &e.jump_host_ids {
            if !seen.insert(h_id) {
                tracing::warn!(target: "smalux::tunnel", "[配置拦截] 隧道 [{}] 跳板链路检测到重复节点成环: {}", e.tunnel_id, h_id);
                e.abort(format!("跳板机链路检测到循环依赖节点: {}", h_id));
                return;
            }
        }

        // 2. 端口转发基础端口校验
        if (e.tunnel_type == "Local" || e.tunnel_type == "Remote") && (e.local_port == 0 || e.remote_port == 0) {
            tracing::warn!(target: "smalux::tunnel", "[配置拦截] 隧道 [{}] 端口配置不合法 (端口号为 0)", e.tunnel_id);
            e.abort("端口转发规则的端口号不能为 0！");
            return;
        }

        tracing::debug!(
            target: "smalux::tunnel",
            "[安全审查:网络隧道保存前置] ID: [{}], 绑定: {}:{}, 远端: {}:{}",
            e.tunnel_id, e.local_bind, e.local_port, e.remote_host, e.remote_port
        );
    });
    g19.detach();

    let g20 = events.global().listen(|e: &crate::event::TunnelBeforeDeleteEvent| {
        if e.is_running {
            tracing::warn!(target: "smalux::tunnel", "[删除拦截] 隧道 [{}] 正在运行中，拒绝物理删除", e.tunnel_id);
            e.abort("该网络规则当前正在运行中，请先停止运行后再删除！");
        }
    });
    g20.detach();

    let g21 = events.global().listen(|e: &crate::event::TunnelMetricsTickEvent| {
        tracing::trace!(
            target: "smalux::tunnel",
            "[流式度量:网络隧道] ID: [{}], 入向: +{} B, 出向: +{} B, 活跃连接: {}",
            e.tunnel_id, e.bytes_in_delta, e.bytes_out_delta, e.active_connections
        );
    });
    g21.detach();
}

impl CoreState {
    /// 默认采用预设种子内存存储创建 CoreState
    pub fn new_mock() -> Self {
        tracing::debug!(target: "smagical_core", "初始化 CoreState 核心状态引擎 (MockStorage 模式)");
        let storage: Arc<dyn AppStorage> = Arc::new(MockStorage::new_seeded());
        let event_manager = Arc::new(EventManager::new());
        attach_default_event_loggers(&event_manager);

        let activity_bar = Arc::new(ActivityBarRegistry::new_with_defaults());
        let right_panels = Arc::new(RwLock::new(RightPanelRegistry::default()));
        let active_terminal = Arc::new(RwLock::new(None));
        let navigation = Arc::new(RwLock::new(NavigationRouter::default()));

        Self {
            storage: Arc::new(RwLock::new(storage)),
            is_mock: Arc::new(RwLock::new(true)),
            event_manager,
            activity_bar,
            right_panels,
            active_terminal,
            navigation,
        }
    }

    /// 使用任意存储后端创建 CoreState
    pub fn new(storage: Arc<dyn AppStorage>) -> Self {
        let event_manager = Arc::new(EventManager::new());
        attach_default_event_loggers(&event_manager);

        let activity_bar = Arc::new(ActivityBarRegistry::new_with_defaults());
        let right_panels = Arc::new(RwLock::new(RightPanelRegistry::default()));
        let active_terminal = Arc::new(RwLock::new(None));
        let navigation = Arc::new(RwLock::new(NavigationRouter::default()));

        Self {
            storage: Arc::new(RwLock::new(storage)),
            is_mock: Arc::new(RwLock::new(false)),
            event_manager,
            activity_bar,
            right_panels,
            active_terminal,
            navigation,
        }
    }

    /// 获取底层存储门面实例句柄
    pub fn storage(&self) -> Arc<dyn AppStorage> {
        self.storage.read().unwrap().clone()
    }

    /// 查询当前是否处于 Mock 数据层模式
    pub fn is_mock_storage(&self) -> bool {
        *self.is_mock.read().unwrap()
    }

    /// 动态切换数据层实现 (Mock 存储 vs 物理持久化数据层)
    pub fn set_mock_storage(&self, enable_mock: bool) {
        let mut is_mock_guard = self.is_mock.write().unwrap();
        *is_mock_guard = enable_mock;
        let mut storage_guard = self.storage.write().unwrap();
        if enable_mock {
            tracing::info!(target: "smagical_core::storage", "数据层已切换至: [MockStorage] 内存种子存储");
            *storage_guard = Arc::new(MockStorage::new_seeded());
        } else {
            tracing::info!(target: "smagical_core::storage", "数据层已切换至: [PhysicalStorage] 物理存储模式 (基线空仓储)");
            *storage_guard = Arc::new(MockStorage::new());
        }
    }

    /// 获取集中式事件管理器引用
    pub fn event_manager(&self) -> &Arc<EventManager> {
        &self.event_manager
    }

    /// 获取全局唯一的共享事件分发器引用 (快捷门面)
    pub fn events(&self) -> &Arc<EventDispatcher> {
        self.event_manager.global()
    }

    /// 获取侧边栏动态注册中心引用
    pub fn activity_bar(&self) -> &Arc<ActivityBarRegistry> {
        &self.activity_bar
    }

    /// 获取右侧辅助面板注册中心引用
    pub fn right_panels(&self) -> &Arc<RwLock<RightPanelRegistry>> {
        &self.right_panels
    }

    /// 获取当前聚焦活跃终端上下文快照
    pub fn active_terminal(&self) -> Option<ActiveTerminalSessionContext> {
        self.active_terminal.read().unwrap().clone()
    }

    /// 切换当前聚焦的活跃终端，并自动广播 `TerminalFocusChangedEvent`
    pub fn set_active_terminal(&self, ctx: Option<ActiveTerminalSessionContext>) {
        let (sess_id, h_id) = if let Some(ref c) = ctx {
            (Some(c.session_id.clone()), c.host_id.clone())
        } else {
            (None, None)
        };
        {
            let mut guard = self.active_terminal.write().unwrap();
            *guard = ctx;
        }
        self.events().dispatch(&TerminalFocusChangedEvent {
            session_id: sess_id,
            host_id: h_id,
        });
    }

    /// 切换右侧辅助面板展开状态，并自动广播 `RightPanelSwitchedEvent`
    pub fn toggle_right_panel(&self, panel_id: &str) -> bool {
        let is_open = {
            let mut guard = self.right_panels.write().unwrap();
            guard.toggle_panel(panel_id)
        };
        self.events().dispatch(&RightPanelSwitchedEvent {
            panel_id: panel_id.to_string(),
            is_open,
        });
        is_open
    }

    /// 动态注册新的右侧辅助伴生面板，并自动广播 `RightPanelRegisteredEvent`
    pub fn register_right_panel(&self, item: crate::domain::RightPanelItem) {
        let item_id = item.id.clone();
        let item_tooltip = item.tooltip.clone();
        {
            let mut guard = self.right_panels.write().unwrap();
            guard.register(item);
        }
        self.events().dispatch(&RightPanelRegisteredEvent {
            panel_id: item_id,
            tooltip: item_tooltip,
        });
    }

    /// 动态注销右侧辅助伴生面板，并自动广播 `RightPanelUnregisteredEvent`
    pub fn unregister_right_panel(&self, panel_id: &str) -> Option<crate::domain::RightPanelItem> {
        let removed = {
            let mut guard = self.right_panels.write().unwrap();
            guard.unregister(panel_id)
        };
        if removed.is_some() {
            self.events().dispatch(&RightPanelUnregisteredEvent {
                panel_id: panel_id.to_string(),
            });
        }
        removed
    }

    /// 向上层当前活动终端发送交互动作 (如执行代码片段)
    pub fn send_terminal_action(&self, session_id: &str, action: TerminalAction) {
        self.events().dispatch(&TerminalActionRequestedEvent {
            session_id: session_id.to_string(),
            action,
        });
    }

    /// 获取导航路由器引用
    pub fn navigation(&self) -> &Arc<RwLock<NavigationRouter>> {
        &self.navigation
    }

    /// 统一发起页面跳转导航，并自动触发生命周期事件
    pub fn navigate_to(&self, request: NavigationRequest) {
        let (prev, curr) = {
            let mut nav = self.navigation.write().unwrap();
            nav.navigate_to(request)
        };

        // 1. 若有上一个激活的模块，触发其失活事件
        if let Some(p) = prev {
            self.events().dispatch(&ModuleDeactivatedEvent {
                target_tab: p.target_tab,
            });
        }

        // 2. 广播全局导航请求事件
        self.events().dispatch(&NavigationRequestedEvent {
            target_tab: curr.target_tab.clone(),
            sub_section: curr.sub_section.clone(),
        });

        // 3. 触发目标模块激活生命周期事件
        self.events().dispatch(&ModuleActivatedEvent {
            target_tab: curr.target_tab,
            sub_section: curr.sub_section,
        });
    }
}

impl Default for CoreState {
    fn default() -> Self {
        Self::new_mock()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_mock_state_has_seeded_groups_and_hosts() {
        let state = CoreState::new_mock();
        let groups = state.storage().groups().list_all().unwrap();
        let hosts = state.storage().hosts().list_all().unwrap();

        assert!(!groups.is_empty());
        assert!(!hosts.is_empty());
        assert_eq!(groups[0].id, "grp-prod");
    }

    #[test]
    fn test_snippet_lifecycle_event_listeners() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use crate::event::{SnippetSavedEvent, SnippetDeletedEvent, SnippetExecutedEvent, SnippetGroupSavedEvent, SnippetGroupDeletedEvent};

        let state = CoreState::new_mock();

        let saved_called = Arc::new(AtomicBool::new(false));
        let deleted_called = Arc::new(AtomicBool::new(false));
        let executed_called = Arc::new(AtomicBool::new(false));
        let grp_saved_called = Arc::new(AtomicBool::new(false));
        let grp_deleted_called = Arc::new(AtomicBool::new(false));

        let s_flag = saved_called.clone();
        let _g1 = state.events().listen(move |e: &SnippetSavedEvent| {
            assert_eq!(e.snippet_id, "snip-test-1");
            assert_eq!(e.title, "Test Docker Snippet");
            s_flag.store(true, Ordering::SeqCst);
        });

        let d_flag = deleted_called.clone();
        let _g2 = state.events().listen(move |e: &SnippetDeletedEvent| {
            assert_eq!(e.snippet_id, "snip-test-1");
            d_flag.store(true, Ordering::SeqCst);
        });

        let e_flag = executed_called.clone();
        let _g3 = state.events().listen(move |e: &SnippetExecutedEvent| {
            assert_eq!(e.snippet_id, "snip-test-1");
            assert_eq!(e.session_id.as_deref(), Some("sess-101"));
            assert!(e.auto_execute);
            e_flag.store(true, Ordering::SeqCst);
        });

        let gs_flag = grp_saved_called.clone();
        let _g4 = state.events().listen(move |e: &SnippetGroupSavedEvent| {
            assert_eq!(e.group_id, "sgrp-ops");
            assert_eq!(e.name, "运维脚本");
            gs_flag.store(true, Ordering::SeqCst);
        });

        let gd_flag = grp_deleted_called.clone();
        let _g5 = state.events().listen(move |e: &SnippetGroupDeletedEvent| {
            assert_eq!(e.group_id, "sgrp-ops");
            gd_flag.store(true, Ordering::SeqCst);
        });

        // 触发分发
        state.events().dispatch(&SnippetSavedEvent {
            snippet_id: "snip-test-1".to_string(),
            title: "Test Docker Snippet".to_string(),
            parent_group_id: Some("sgrp-ops".to_string()),
            is_new: true,
        });

        state.events().dispatch(&SnippetExecutedEvent {
            snippet_id: "snip-test-1".to_string(),
            session_id: Some("sess-101".to_string()),
            auto_execute: true,
        });

        state.events().dispatch(&SnippetDeletedEvent {
            snippet_id: "snip-test-1".to_string(),
        });

        state.events().dispatch(&SnippetGroupSavedEvent {
            group_id: "sgrp-ops".to_string(),
            name: "运维脚本".to_string(),
            parent_id: None,
            is_new: true,
        });

        state.events().dispatch(&SnippetGroupDeletedEvent {
            group_id: "sgrp-ops".to_string(),
        });

        assert!(saved_called.load(Ordering::SeqCst));
        assert!(deleted_called.load(Ordering::SeqCst));
        assert!(executed_called.load(Ordering::SeqCst));
        assert!(grp_saved_called.load(Ordering::SeqCst));
        assert!(grp_deleted_called.load(Ordering::SeqCst));
    }
}
