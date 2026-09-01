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
    storage: Arc<dyn AppStorage>,
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
            storage,
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
            storage,
            event_manager,
            activity_bar,
            right_panels,
            active_terminal,
            navigation,
        }
    }

    /// 获取底层存储门面引用
    pub fn storage(&self) -> &Arc<dyn AppStorage> {
        &self.storage
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
}
