//! 统一视图生命周期管理与内存按需卸载调度中枢 (View Lifecycle Manager)。
//!
//! 核心设计理念：
//! 1. 按需灌入 (Universal Lazy-Injected UI)：视图/抽屉激活时微秒级将 Rust 内存数据灌入 Slint；
//! 2. 即关即卸 (Unload on Hide)：视图关闭或切离时，清空 Slint 绑定的 ModelRc，释放 UI 堆内存与观察者；
//! 3. 状态零丢失 (Persistent Rust State)：分组展开折叠、搜索过滤词、当前路径等均由 Rust 纯数据层托管，重开时微秒级无缝回填；
//! 4. 后台长任务零中断 (Zero Disruption)：SSH 终端、SFTP 传输、网络隧道、AI 流完全由 Tokio 异步后台守护，与 UI 进退彻底解耦。

use std::cell::RefCell;
use std::rc::Rc;
use slint::{ComponentHandle, ModelRc};

use crate::generated::{
    AppWindow, CredentialsBridge, FilesBridge, HistoryBridge, HostsBridge, SnippetsBridge,
    TunnelsBridge, WindowBridge,
};
use crate::handlers::AppContext;

/// 视图生命周期调度器
pub(crate) struct ViewLifecycleManager {
    last_main_view: RefCell<String>,
    last_left_tab: RefCell<String>,
    is_left_drawer_open: RefCell<bool>,
    window: slint::Weak<AppWindow>,
    ctx: AppContext,
}

impl ViewLifecycleManager {
    /// 创建全新的视图生命周期调度器实例
    pub fn new(window: slint::Weak<AppWindow>, ctx: AppContext) -> Rc<Self> {
        Rc::new(Self {
            last_main_view: RefCell::new("terminal".to_string()),
            last_left_tab: RefCell::new("hosts".to_string()),
            is_left_drawer_open: RefCell::new(true),
            window,
            ctx,
        })
    }

    /// 统一同步并处理当前所有视图/抽屉的生命周期变化 (自动检测主视图与左侧抽屉状态差分)
    pub fn sync_lifecycle(&self) {
        let Some(w) = self.window.upgrade() else { return };
        let wb = w.global::<WindowBridge>();
        let cur_main_view = wb.get_main_view().to_string();
        let cur_left_tab = wb.get_active_left_tab().to_string();
        let cur_drawer_open = wb.get_is_left_drawer_open();

        self.on_main_view_changed(&cur_main_view);
        self.on_left_drawer_changed(cur_drawer_open, &cur_left_tab);
    }

    /// 主视图切换调度 (main-view: "terminal" | "files" | "history" | "credentials" | "snippets" | "tunnels" | "settings")
    pub fn on_main_view_changed(&self, new_view: &str) {
        let old_view = self.last_main_view.borrow().clone();
        if old_view == new_view {
            return;
        }

        tracing::info!(target: "smalux::lifecycle", "主视图路由切换: [{}] -> [{}]", old_view, new_view);

        // 如果切离 terminal 工作区且右侧伴生抽屉处于打开状态，级联静默卸载右侧伴生模型
        if old_view == "terminal" && new_view != "terminal" {
            if let Some(w) = self.window.upgrade() {
                let wb = w.global::<WindowBridge>();
                if wb.get_is_right_drawer_open() {
                    let cur_tool = wb.get_active_right_tool().to_string();
                    crate::handlers::right_drawer_handlers::unload_right_tool_models(&w, &cur_tool);
                    tracing::debug!(target: "smalux::lifecycle", "切离终端主视图，级联卸载右侧伴生抽屉 [{}] 数据模型", cur_tool);
                }
            }
        }

        // 1. 切离旧主视图：就地卸载对应页面的 Slint 适配层 Model
        self.unload_main_view(&old_view);

        // 2. 切入新主视图：按需从 Rust 内存秒级回填 Slint Model
        self.load_main_view(new_view);

        // 如果重回 terminal 工作区且右侧伴生抽屉原本处于打开状态，级联微秒级恢复右侧伴生模型
        if old_view != "terminal" && new_view == "terminal" {
            if let Some(w) = self.window.upgrade() {
                let wb = w.global::<WindowBridge>();
                if wb.get_is_right_drawer_open() {
                    let cur_tool = wb.get_active_right_tool().to_string();
                    crate::handlers::right_drawer_handlers::load_right_tool_models(&w, &cur_tool);
                    tracing::debug!(target: "smalux::lifecycle", "重回终端主视图，级联恢复右侧伴生抽屉 [{}] 数据模型", cur_tool);
                }
            }
        }

        *self.last_main_view.borrow_mut() = new_view.to_string();
    }

    /// 左侧抽屉状态切换调度 (is_open, active_tab: "hosts" | "credentials" | "snippets" | "tunnels" | "backup" | "settings")
    pub fn on_left_drawer_changed(&self, is_open: bool, active_tab: &str) {
        let prev_open = *self.is_left_drawer_open.borrow();
        let prev_tab = self.last_left_tab.borrow().clone();

        // 状态未发生任何变化，直接返回
        if prev_open == is_open && prev_tab == active_tab {
            return;
        }

        tracing::debug!(
            target: "smalux::lifecycle",
            "左侧抽屉状态调度: open [{} -> {}], tab [{} -> {}]",
            prev_open, is_open, prev_tab, active_tab
        );

        if !is_open {
            // 抽屉收起：卸载当前抽屉对应的 Slint 绑定的模型
            self.unload_left_drawer(&prev_tab);
        } else {
            // 抽屉展开或切换 Tab
            if prev_open && prev_tab != active_tab {
                // 如果在 hosts 和 files 之间切换，二者共享 HostsDrawer，避免冗余清空与重绘
                let both_hosts_drawer = (prev_tab == "hosts" || prev_tab == "files")
                    && (active_tab == "hosts" || active_tab == "files");
                if !both_hosts_drawer {
                    self.unload_left_drawer(&prev_tab);
                }
            }
            // 加载当前展开 Tab 的模型
            self.load_left_drawer(active_tab);
        }

        *self.is_left_drawer_open.borrow_mut() = is_open;
        *self.last_left_tab.borrow_mut() = active_tab.to_string();
    }

    // =========================================================================
    // 主视图卸载与按需加载核心逻辑
    // =========================================================================

    fn unload_main_view(&self, view: &str) {
        let Some(w) = self.window.upgrade() else { return };
        match view {
            "files" => {
                let fb = w.global::<FilesBridge>();
                fb.set_local_files(ModelRc::default());
                fb.set_remote_files(ModelRc::default());
                fb.set_file_launcher_host_items(ModelRc::default());
                tracing::debug!(target: "smalux::lifecycle", "已释放 Files 页面 Slint 文件模型与选择弹窗模型");
            }
            "history" => {
                let hb = w.global::<HistoryBridge>();
                hb.set_history_groups(ModelRc::default());
                tracing::debug!(target: "smalux::lifecycle", "已释放 History 页面 Slint 分组模型");
            }
            "credentials" => {
                let cb = w.global::<CredentialsBridge>();
                cb.set_credentials(ModelRc::default());
                tracing::debug!(target: "smalux::lifecycle", "已释放 Credentials 页面 Slint 凭据模型 (敏感数据脱敏)");
            }
            "snippets" => {
                let sb = w.global::<SnippetsBridge>();
                sb.set_tree_nodes(ModelRc::default());
                sb.set_quick_cmds(ModelRc::default());
                sb.set_parent_options(ModelRc::default());
                tracing::debug!(target: "smalux::lifecycle", "已释放 Snippets 页面 Slint 代码片段模型");
            }
            "tunnels" => {
                let tb = w.global::<TunnelsBridge>();
                tb.set_tunnels(ModelRc::default());
                tb.set_form_hops(ModelRc::default());
                tb.set_host_tunnels(ModelRc::default());
                tracing::debug!(target: "smalux::lifecycle", "已释放 Tunnels 页面 Slint 隧道模型");
            }
            "terminal" => {
                // 切离终端工作区时，后台 PTY 照常运行
                tracing::debug!(target: "smalux::lifecycle", "切离终端工作区，保持后台 PTY 进程与会话状态");
            }
            _ => {}
        }
    }

    fn load_main_view(&self, view: &str) {
        let Some(w) = self.window.upgrade() else { return };
        match view {
            "files" => {
                crate::handlers::file_handlers::sync_file_explorer_ui(&w, &self.ctx);
                tracing::debug!(target: "smalux::lifecycle", "已重新绑定 Files 页面 Slint 数据模型");
            }
            "history" => {
                crate::handlers::history_handlers::sync_ui_history(&w, &self.ctx);
                tracing::debug!(target: "smalux::lifecycle", "已重新绑定 History 页面 Slint 数据模型");
            }
            "credentials" => {
                crate::handlers::credential_handlers::sync_credentials_ui(&w, &self.ctx.core_state, "all", "");
                tracing::debug!(target: "smalux::lifecycle", "已重新绑定 Credentials 页面 Slint 凭据模型");
            }
            "snippets" => {
                crate::handlers::snippet_handlers::sync_ui_snippets(&w, &self.ctx);
                tracing::debug!(target: "smalux::lifecycle", "已重新绑定 Snippets 页面 Slint 代码片段模型");
            }
            "tunnels" => {
                crate::handlers::tunnel_handlers::sync_ui_tunnels(&w, &self.ctx);
                tracing::debug!(target: "smalux::lifecycle", "已重新绑定 Tunnels 页面 Slint 隧道模型");
            }
            "terminal" => {
                // 切回终端工作区时，即时同步恢复当前活跃终端会话与屏幕渲染
                let pane_groups = self.ctx.pane_groups.borrow();
                let active_pane_id = self.ctx.active_pane_id.borrow();
                let is_split = self.ctx.global_split_tree.borrow().is_some();
                crate::session::sync_active_session_ui(&w, &pane_groups, &active_pane_id, is_split);
                if let Ok(mut terminals) = self.ctx.active_terminals.try_borrow_mut() {
                    for g in pane_groups.iter() {
                        if let Some(sess) = g.get_active_session() {
                            if let Some(inst) = terminals.get_mut(&sess.session_id) {
                                inst.parser.mark_dirty();
                            }
                        }
                    }
                }
                tracing::debug!(target: "smalux::lifecycle", "已恢复终端工作区活跃会话与屏幕渲染");
            }
            _ => {}
        }
    }

    // =========================================================================
    // 抽屉卸载与按需加载核心逻辑
    // =========================================================================

    fn unload_left_drawer(&self, tab: &str) {
        let Some(w) = self.window.upgrade() else { return };
        match tab {
            "hosts" | "files" => {
                let hb = w.global::<HostsBridge>();
                hb.set_tree_nodes(ModelRc::default());
                hb.set_hosts(ModelRc::default());
                tracing::debug!(target: "smalux::lifecycle", "已清空 Hosts 抽屉 Slint 树形与卡片模型 (tab: {})", tab);
            }
            "credentials" => {
                let cb = w.global::<CredentialsBridge>();
                cb.set_credentials(ModelRc::default());
                tracing::debug!(target: "smalux::lifecycle", "已清空 Credentials 抽屉 Slint 模型");
            }
            "snippets" => {
                let sb = w.global::<SnippetsBridge>();
                sb.set_tree_nodes(ModelRc::default());
                sb.set_quick_cmds(ModelRc::default());
                sb.set_parent_options(ModelRc::default());
                tracing::debug!(target: "smalux::lifecycle", "已清空 Snippets 抽屉 Slint 模型");
            }
            "tunnels" => {
                let tb = w.global::<TunnelsBridge>();
                tb.set_tunnels(ModelRc::default());
                tb.set_form_hops(ModelRc::default());
                tb.set_host_tunnels(ModelRc::default());
                tracing::debug!(target: "smalux::lifecycle", "已清空 Tunnels 抽屉 Slint 模型");
            }
            _ => {}
        }
    }

    fn load_left_drawer(&self, tab: &str) {
        let Some(w) = self.window.upgrade() else { return };
        match tab {
            "hosts" | "files" => {
                // 从 HostStore 内存极速回填 (< 0.5ms)，展开折叠状态与搜索词完全不丢
                self.ctx.host_store.schedule_tree_refresh(self.window.clone());
                tracing::debug!(target: "smalux::lifecycle", "已从 HostStore 毫秒级回填 Hosts 树形模型 (tab: {})", tab);
            }
            "credentials" => {
                crate::handlers::credential_handlers::sync_credentials_ui(&w, &self.ctx.core_state, "all", "");
                tracing::debug!(target: "smalux::lifecycle", "已回填 Credentials 抽屉模型");
            }
            "snippets" => {
                crate::handlers::snippet_handlers::sync_ui_snippets(&w, &self.ctx);
                tracing::debug!(target: "smalux::lifecycle", "已回填 Snippets 抽屉模型");
            }
            "tunnels" => {
                crate::handlers::tunnel_handlers::sync_ui_tunnels(&w, &self.ctx);
                tracing::debug!(target: "smalux::lifecycle", "已回填 Tunnels 伴生抽屉模型");
            }
            _ => {}
        }
    }
}
