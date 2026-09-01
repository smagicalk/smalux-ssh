//! 全局应用级 Hook 调度引擎与动态热插拔监听者管理器。

use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use crate::domain::{
    ActiveTerminalSessionContext, GroupRecord, HistoryRecord, HostRecord, HostStatus,
    NavigationRequest, RightPanelItem, TerminalAction,
};
use crate::hook::HookDecision;
use super::builtin::FunctionalGlobalHook;
use super::traits::AppGlobalHook;
use super::types::{AppBootContext, AppExitContext, ConfigChangeEvent, WindowState};


/// 动态监听订阅 RAII 句柄 (持有弱引用，离开作用域销毁时自动触发注销，彻底杜绝内存泄漏)。
pub struct ListenerHandle {
    id: u64,
    engine: std::sync::Weak<AppGlobalHookEngine>,
    detached: bool,
}

impl ListenerHandle {
    /// 创建新句柄实例
    pub(crate) fn new(id: u64, engine: std::sync::Weak<AppGlobalHookEngine>) -> Self {
        Self {
            id,
            engine,
            detached: false,
        }
    }

    /// 获取该订阅的注册 ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// 脱离自动注销管理，使该监听在后台永久常驻运行
    pub fn detach(mut self) {
        self.detached = true;
    }

    /// 显式注销并拔除该 Hook 监听器
    pub fn unregister(mut self) {
        self.perform_unregister();
        self.detached = true;
    }

    fn perform_unregister(&self) {
        if let Some(engine) = self.engine.upgrade() {
            engine.unregister_by_id(self.id);
        }
    }
}

impl Drop for ListenerHandle {
    fn drop(&mut self) {
        if !self.detached {
            self.perform_unregister();
        }
    }
}

/// 内部插件注册包装项
struct AppHookEntry {
    id: u64,
    hook: Arc<dyn AppGlobalHook>,
}

/// 全局应用级 Hook 统一调度与动态注册引擎 (线程安全，支持物理隔离与动态热插拔)。
pub struct AppGlobalHookEngine {
    hooks: RwLock<Vec<AppHookEntry>>,
    next_id: AtomicU64,
}

impl Default for AppGlobalHookEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AppGlobalHookEngine {
    /// 创建一个新的全局应用 Hook 调度引擎实例。
    pub fn new() -> Self {
        Self {
            hooks: RwLock::new(Vec::new()),
            next_id: AtomicU64::new(1),
        }
    }

    /// 动态注册一个外部全局 Hook 监听器 (按优先级降序排序，数值越大越先执行)，返回唯一注册 ID。
    pub fn register(&self, hook: Arc<dyn AppGlobalHook>) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let mut list = self.hooks.write().unwrap_or_else(|e| e.into_inner());
        list.retain(|entry| entry.hook.name() != hook.name());
        list.push(AppHookEntry { id, hook });
        list.sort_by_key(|b| std::cmp::Reverse(b.hook.priority()));
        id
    }

    /// 动态注册并通过句柄管理生命周期。
    pub fn register_with_handle(self: &Arc<Self>, hook: Arc<dyn AppGlobalHook>) -> ListenerHandle {
        let id = self.register(hook);
        ListenerHandle::new(id, Arc::downgrade(self))
    }


    /// 快速挂载一个基于闭包的全局配置变动监听器。
    pub fn on_config_changed<F>(self: &Arc<Self>, f: F) -> ListenerHandle
    where
        F: Fn(&ConfigChangeEvent) + Send + Sync + 'static,
    {
        let hook = Arc::new(FunctionalGlobalHook::for_config_changed("closure_config_listener", f));
        self.register_with_handle(hook)
    }

    /// 快速挂载一个应用关闭前拦截询问闭包守卫。
    pub fn on_app_before_exit<F>(self: &Arc<Self>, priority: i32, f: F) -> ListenerHandle
    where
        F: Fn(&AppExitContext) -> HookDecision + Send + Sync + 'static,
    {
        let hook = Arc::new(FunctionalGlobalHook::for_before_exit("closure_before_exit_guard", priority, f));
        self.register_with_handle(hook)
    }

    /// 动态注销指定名称的全局 Hook 监听器。
    pub fn unregister(&self, name: &str) {
        let mut list = self.hooks.write().unwrap_or_else(|e| e.into_inner());
        list.retain(|entry| entry.hook.name() != name);
    }

    /// 根据唯一 ID 动态注销全局 Hook 监听器。
    pub fn unregister_by_id(&self, id: u64) {
        let mut list = self.hooks.write().unwrap_or_else(|e| e.into_inner());
        list.retain(|entry| entry.id != id);
    }

    /// 获取当前所有已注册全局 Hook 的名称列表。
    pub fn list_hooks(&self) -> Vec<&'static str> {
        let list = self.hooks.read().unwrap_or_else(|e| e.into_inner());
        list.iter().map(|entry| entry.hook.name()).collect()
    }

    /// 获取已注册全局 Hook 的总数量。
    pub fn len(&self) -> usize {
        let list = self.hooks.read().unwrap_or_else(|e| e.into_inner());
        list.len()
    }

    /// 判断是否没有任何全局 Hook 注册。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // =========================================================================
    // 1. 进程级生命周期 (Process Lifecycle - app_)
    // =========================================================================

    /// 广播应用启动引导事件 (`on_app_boot`)。
    pub fn dispatch_app_boot(&self, ctx: &AppBootContext) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let ctx_cloned = ctx.clone();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_app_boot(&ctx_cloned);
            }));
        }
    }

    /// 广播主界面首帧就绪事件 (`on_app_ready`)。
    pub fn dispatch_app_ready(&self) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_app_ready();
            }));
        }
    }

    /// 广播应用退出前拦截询问 (`on_app_before_exit`)。
    pub fn dispatch_app_before_exit(&self, ctx: &AppExitContext) -> HookDecision {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let ctx_cloned = ctx.clone();
            let res = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_app_before_exit(&ctx_cloned)
            }));
            if let Ok(decision) = res
                && matches!(decision, HookDecision::Abort { .. })
            {
                return decision;
            }
        }
        HookDecision::Continue
    }

    /// 广播应用完全退出事件 (`on_app_exit`)。
    pub fn dispatch_app_exit(&self, ctx: &AppExitContext) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let ctx_cloned = ctx.clone();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_app_exit(&ctx_cloned);
            }));
        }
    }

    // =========================================================================
    // 2. 框架外壳与全局导航域 (Shell & Navigation - shell_)
    // =========================================================================

    /// 广播页面跳转导航请求 (`on_shell_navigation_requested`)。
    pub fn dispatch_shell_navigation_requested(&self, req: &NavigationRequest) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let req_cloned = req.clone();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_shell_navigation_requested(&req_cloned);
            }));
        }
    }

    /// 广播模块激活挂载事件 (`on_shell_module_activated`)。
    pub fn dispatch_shell_module_activated(
        &self,
        tab_id: &str,
        sub_section: Option<&str>,
        params: &std::collections::HashMap<String, String>,
    ) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let t_id = tab_id.to_string();
            let sub_sec = sub_section.map(|s| s.to_string());
            let p = params.clone();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_shell_module_activated(&t_id, sub_sec.as_deref(), &p);
            }));
        }
    }

    /// 广播模块失活休眠事件 (`on_shell_module_deactivated`)。
    pub fn dispatch_shell_module_deactivated(&self, tab_id: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let t_id = tab_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_shell_module_deactivated(&t_id);
            }));
        }
    }

    /// 广播左侧活动栏菜单点击事件 (`on_shell_left_menu_clicked`)。
    pub fn dispatch_shell_left_menu_clicked(&self, menu_id: &str, old_menu_id: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let m_id = menu_id.to_string();
            let old_id = old_menu_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_shell_left_menu_clicked(&m_id, &old_id);
            }));
        }
    }

    /// 广播主工作区视图流转事件 (`on_shell_main_view_switched`)。
    pub fn dispatch_shell_main_view_switched(&self, current_view: &str, previous_view: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let cur = current_view.to_string();
            let prev = previous_view.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_shell_main_view_switched(&cur, &prev);
            }));
        }
    }

    /// 广播全局快捷指令执行事件 (`on_shell_command_executed`)。
    pub fn dispatch_shell_command_executed(&self, command_id: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let cmd = command_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_shell_command_executed(&cmd);
            }));
        }
    }

    /// 广播全局模态弹窗显隐事件 (`on_shell_modal_toggled`)。
    pub fn dispatch_shell_modal_toggled(&self, modal_id: &str, is_open: bool) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let m_id = modal_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_shell_modal_toggled(&m_id, is_open);
            }));
        }
    }

    /// 广播窗口全局状态变动事件 (`on_shell_window_state_changed`)。
    pub fn dispatch_shell_window_state_changed(&self, state: WindowState) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_shell_window_state_changed(state);
            }));
        }
    }

    // =========================================================================
    // 3. 左侧主机资产抽屉域 (Left Column - host_asset_)
    // =========================================================================

    /// 广播主机资产创建成功事件 (`on_host_asset_created`)。
    pub fn dispatch_host_asset_created(&self, host: &HostRecord) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let h = host.clone();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_asset_created(&h);
            }));
        }
    }

    /// 广播主机资产更新修改事件 (`on_host_asset_updated`)。
    pub fn dispatch_host_asset_updated(&self, old_host: &HostRecord, new_host: &HostRecord) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let old = old_host.clone();
            let new = new_host.clone();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_asset_updated(&old, &new);
            }));
        }
    }

    /// 广播主机资产删除事件 (`on_host_asset_deleted`)。
    pub fn dispatch_host_asset_deleted(&self, host_id: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let id = host_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_asset_deleted(&id);
            }));
        }
    }

    /// 广播主机分组创建事件 (`on_host_asset_group_created`)。
    pub fn dispatch_host_asset_group_created(&self, group: &GroupRecord) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let g = group.clone();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_asset_group_created(&g);
            }));
        }
    }

    /// 广播主机分组更新事件 (`on_host_asset_group_updated`)。
    pub fn dispatch_host_asset_group_updated(&self, group: &GroupRecord) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let g = group.clone();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_asset_group_updated(&g);
            }));
        }
    }

    /// 广播主机分组删除事件 (`on_host_asset_group_deleted`)。
    pub fn dispatch_host_asset_group_deleted(&self, group_id: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let id = group_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_asset_group_deleted(&id);
            }));
        }
    }

    /// 广播主机分组折叠/展开事件 (`on_host_asset_group_toggled`)。
    pub fn dispatch_host_asset_group_toggled(&self, group_id: &str, is_expanded: bool) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let id = group_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_asset_group_toggled(&id, is_expanded);
            }));
        }
    }

    /// 广播主机树节点拖拽调序事件 (`on_host_asset_tree_reordered`)。
    pub fn dispatch_host_asset_tree_reordered(&self, src_id: &str, target_id: &str, drop_position: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let s = src_id.to_string();
            let t = target_id.to_string();
            let pos = drop_position.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_asset_tree_reordered(&s, &t, &pos);
            }));
        }
    }

    /// 广播主机资产搜索过滤事件 (`on_host_asset_search_filtered`)。
    pub fn dispatch_host_asset_search_filtered(&self, query: &str, match_count: usize) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let q = query.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_asset_search_filtered(&q, match_count);
            }));
        }
    }

    /// 广播主机卡片单击选中预览事件 (`on_host_asset_selected_for_preview`)。
    pub fn dispatch_host_asset_selected_for_preview(&self, host: Option<&HostRecord>) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let h = host.cloned();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_asset_selected_for_preview(h.as_ref());
            }));
        }
    }

    /// 广播主机后台探活状态更新事件 (`on_host_asset_status_probed`)。
    pub fn dispatch_host_asset_status_probed(&self, host_id: &str, status: HostStatus, ping_ms: i32) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let id = host_id.to_string();
            let st = status.clone();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_asset_status_probed(&id, st, ping_ms);
            }));
        }
    }


    // =========================================================================
    // 4. 中央终端工作区域 (Center Column - host_terminal_)
    // =========================================================================

    /// 广播终端打开前拦截询问 (`on_host_terminal_opening`)。
    pub fn dispatch_host_terminal_opening(&self, host_id: &str, is_local: bool) -> HookDecision {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let h_id = host_id.to_string();
            let res = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_terminal_opening(&h_id, is_local)
            }));
            if let Ok(decision) = res && !decision.is_continue() {
                return decision;
            }
        }
        HookDecision::Continue
    }

    /// 广播终端会话建立就绪事件 (`on_host_terminal_opened`)。
    pub fn dispatch_host_terminal_opened(&self, session_id: &str, ctx: &ActiveTerminalSessionContext) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let s_id = session_id.to_string();
            let c = ctx.clone();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_terminal_opened(&s_id, &c);
            }));
        }
    }

    /// 广播终端聚焦会话变更事件 (`on_host_terminal_focus_changed`)。
    pub fn dispatch_host_terminal_focus_changed(&self, ctx: Option<&ActiveTerminalSessionContext>) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let ctx_cloned = ctx.cloned();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_terminal_focus_changed(ctx_cloned.as_ref());
            }));
        }
    }

    /// 广播终端多分屏拓扑变更事件 (`on_host_terminal_split_changed`)。
    pub fn dispatch_host_terminal_split_changed(&self, pane_count: usize, active_pane_id: &str, is_split: bool) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let pid = active_pane_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_terminal_split_changed(pane_count, &pid, is_split);
            }));
        }
    }

    /// 广播终端标题重命名事件 (`on_host_terminal_title_renamed`)。
    pub fn dispatch_host_terminal_title_renamed(&self, session_id: &str, new_title: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let sid = session_id.to_string();
            let title = new_title.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_terminal_title_renamed(&sid, &title);
            }));
        }
    }

    /// 广播终端响铃提醒事件 (`on_host_terminal_bell_triggered`)。
    pub fn dispatch_host_terminal_bell_triggered(&self, session_id: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let sid = session_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_terminal_bell_triggered(&sid);
            }));
        }
    }

    /// 广播终端关闭前守护询问 (`on_host_terminal_closing`)。
    pub fn dispatch_host_terminal_closing(&self, session_id: &str) -> HookDecision {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let sid = session_id.to_string();
            let res = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_terminal_closing(&sid)
            }));
            if let Ok(decision) = res && !decision.is_continue() {
                return decision;
            }
        }
        HookDecision::Continue
    }

    /// 广播终端会话彻底销毁事件 (`on_host_terminal_closed`)。
    pub fn dispatch_host_terminal_closed(&self, session_id: &str, duration_secs: u64) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let sid = session_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_terminal_closed(&sid, duration_secs);
            }));
        }
    }

    // =========================================================================
    // 5. 右侧辅助伴生抽屉域 (Right Column - host_right_)
    // =========================================================================

    /// 广播右侧伴生抽屉展开/折叠事件 (`on_host_right_drawer_toggled`)。
    pub fn dispatch_host_right_drawer_toggled(&self, is_open: bool, active_panel_id: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let pid = active_panel_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_right_drawer_toggled(is_open, &pid);
            }));
        }
    }

    /// 广播右侧伴生抽屉拖拽调整宽度事件 (`on_host_right_drawer_resized`)。
    pub fn dispatch_host_right_drawer_resized(&self, width: f32) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_right_drawer_resized(width);
            }));
        }
    }

    /// 广播右侧伴生面板切换事件 (`on_host_right_panel_switched`)。
    pub fn dispatch_host_right_panel_switched(&self, panel_id: &str, is_open: bool) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let p_id = panel_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_right_panel_switched(&p_id, is_open);
            }));
        }
    }

    /// 广播右侧伴生插件动态注册事件 (`on_host_right_panel_registered`)。
    pub fn dispatch_host_right_panel_registered(&self, item: &RightPanelItem) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let it = item.clone();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_right_panel_registered(&it);
            }));
        }
    }

    /// 广播右侧伴生插件注销事件 (`on_host_right_panel_unregistered`)。
    pub fn dispatch_host_right_panel_unregistered(&self, panel_id: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let pid = panel_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_right_panel_unregistered(&pid);
            }));
        }
    }

    /// 广播终端指令/动作执行请求 (`on_host_terminal_action_requested`)。
    pub fn dispatch_host_terminal_action_requested(&self, session_id: &str, action: &TerminalAction) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let s_id = session_id.to_string();
            let act = action.clone();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_terminal_action_requested(&s_id, &act);
            }));
        }
    }

    /// 广播右侧 SFTP 穿透目录同步请求 (`on_host_right_sftp_sync_requested`)。
    pub fn dispatch_host_right_sftp_sync_requested(&self, session_id: &str, remote_path: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let sid = session_id.to_string();
            let path = remote_path.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_host_right_sftp_sync_requested(&sid, &path);
            }));
        }
    }

    // =========================================================================
    // 6. 历史会话中心域 (history_)
    // =========================================================================

    /// 广播会话历史记录沉淀事件 (`on_history_session_recorded`)。
    pub fn dispatch_history_session_recorded(&self, history: &HistoryRecord) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let h = history.clone();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_history_session_recorded(&h);
            }));
        }
    }

    /// 广播单条历史项删除事件 (`on_history_item_deleted`)。
    pub fn dispatch_history_item_deleted(&self, history_id: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let hid = history_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_history_item_deleted(&hid);
            }));
        }
    }

    /// 广播会话历史一键清空事件 (`on_history_cleared`)。
    pub fn dispatch_history_cleared(&self) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_history_cleared();
            }));
        }
    }

    /// 广播会话历史置顶切换事件 (`on_history_pin_toggled`)。
    pub fn dispatch_history_pin_toggled(&self, history_id: &str, is_pinned: bool) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let hid = history_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_history_pin_toggled(&hid, is_pinned);
            }));
        }
    }

    /// 广播从历史记录发起重新连接事件 (`on_history_reconnect_requested`)。
    pub fn dispatch_history_reconnect_requested(&self, history_id: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let hid = history_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_history_reconnect_requested(&hid);
            }));
        }
    }

    // =========================================================================
    // 7. 凭据与密钥管理域 (credential_)
    // =========================================================================

    /// 广播 SSH 密钥/凭据创建事件 (`on_credential_created`)。
    pub fn dispatch_credential_created(&self, cred_id: &str, name: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let cid = cred_id.to_string();
            let n = name.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_credential_created(&cid, &n);
            }));
        }
    }

    /// 广播 SSH 密钥/凭据更新事件 (`on_credential_updated`)。
    pub fn dispatch_credential_updated(&self, cred_id: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let cid = cred_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_credential_updated(&cid);
            }));
        }
    }

    /// 广播 SSH 密钥/凭据删除事件 (`on_credential_deleted`)。
    pub fn dispatch_credential_deleted(&self, cred_id: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let cid = cred_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_credential_deleted(&cid);
            }));
        }
    }

    // =========================================================================
    // 8. 运维代码片段域 (snippet_)
    // =========================================================================

    /// 广播代码片段创建事件 (`on_snippet_created`)。
    pub fn dispatch_snippet_created(&self, snippet_id: &str, title: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let sid = snippet_id.to_string();
            let t = title.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_snippet_created(&sid, &t);
            }));
        }
    }

    /// 广播代码片段更新事件 (`on_snippet_updated`)。
    pub fn dispatch_snippet_updated(&self, snippet_id: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let sid = snippet_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_snippet_updated(&sid);
            }));
        }
    }

    /// 广播代码片段删除事件 (`on_snippet_deleted`)。
    pub fn dispatch_snippet_deleted(&self, snippet_id: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let sid = snippet_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_snippet_deleted(&sid);
            }));
        }
    }

    /// 广播代码片段一键执行事件 (`on_snippet_executed`)。
    pub fn dispatch_snippet_executed(&self, snippet_id: &str, session_id: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let snid = snippet_id.to_string();
            let seid = session_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_snippet_executed(&snid, &seid);
            }));
        }
    }

    // =========================================================================
    // 9. 设置、主题与配置变更域 (config_ / theme_)
    // =========================================================================

    /// 广播全局参数变更事件 (`on_config_changed`)。
    pub fn dispatch_config_changed(&self, event: &ConfigChangeEvent) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let ev = event.clone();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_config_changed(&ev);
            }));
        }
    }

    /// 广播配置恢复默认预设事件 (`on_config_reset`)。
    pub fn dispatch_config_reset(&self, section: &str) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let sec = section.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_config_reset(&sec);
            }));
        }
    }

    /// 广播深浅色外观切换事件 (`on_theme_mode_toggled`)。
    pub fn dispatch_theme_mode_toggled(&self, is_dark: bool) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_theme_mode_toggled(is_dark);
            }));
        }
    }

    /// 广播预设主题切换事件 (`on_theme_changed`)。
    pub fn dispatch_theme_changed(&self, theme_id: &str, is_dark: bool) {
        let entries = { self.hooks.read().unwrap().iter().map(|e| Arc::clone(&e.hook)).collect::<Vec<_>>() };
        for hook in entries {
            let hook_cloned = Arc::clone(&hook);
            let tid = theme_id.to_string();
            let _ = panic::catch_unwind(AssertUnwindSafe(move || {
                hook_cloned.on_theme_changed(&tid, is_dark);
            }));
        }
    }
}
