//! UI 事件回调与业务路由层。
//!
//! 将 Slint UI 各区域的回调绑定按功能域拆分为独立的处理器模块。

pub(crate) mod debug_handlers;
pub(crate) mod host_handlers;
pub(crate) mod session_handlers;
pub(crate) mod window_handlers;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use smagical_core::theme::ThemeService;
use smagical_core::CoreState;

use crate::generated::{AppWindow, HostItemData, LocalShellItemData};
use crate::session::TerminalSessionInfo;
use crate::terminal::TerminalInstance;
use crate::tree_model::RawTreeNode;


/// 应用全局共享上下文句柄集合。
///
/// 封装主窗口所有状态引用的生命周期共享所有权，提供给各个子处理器模块使用。
#[derive(Clone)]
pub(crate) struct AppContext {
    /// 核心状态与底层存储引擎门面 (CoreState & Storage)
    pub core_state: Rc<CoreState>,
    /// 内存全量主机/分组树形节点镜像缓存 (Master Tree)
    pub master_tree: Rc<RefCell<Vec<RawTreeNode>>>,
    /// 内存全量卡片模式主机列表数据缓存 (Master Cards)
    pub master_cards: Rc<RefCell<Vec<HostItemData>>>,
    /// 树形视图当前已展开的分组 ID 集合 (Expanded Group IDs)
    pub expanded_groups: Rc<RefCell<HashSet<String>>>,
    /// 新建/编辑主机弹窗中上级分组树选择器已展开的分组 ID 集合
    pub selector_expanded_groups: Rc<RefCell<HashSet<String>>>,
    /// 侧边栏主机搜索栏当前输入的过滤关键词 (Search Query)
    pub search_query: Rc<RefCell<String>>,
    /// 当前运行时已打开并管理的终端会话 UI 描述列表 (Active Terminal Sessions UI metadata)
    pub active_sessions: Rc<RefCell<Vec<TerminalSessionInfo>>>,
    /// 运行中的终端底层会话实例表 (Session ID -> TerminalInstance)
    pub active_terminals: Rc<RefCell<HashMap<String, TerminalInstance>>>,
    /// 新增终端会话的自增序号计数器 (Next Session Sequence ID)
    pub next_session_num: Rc<RefCell<usize>>,
    /// 启动时探测到的本地可用 Shell 环境列表只读缓存 (Cached Local Shells)
    pub cached_shells: Rc<Vec<LocalShellItemData>>,
    /// 全局配色主题管理器服务 (Theme Service)
    pub themes: Rc<ThemeService>,
    /// 终端像素帧光栅化渲染器实例 (Terminal Renderer)
    pub terminal_renderer: Rc<RefCell<Option<crate::terminal::TerminalRenderer>>>,
    /// 会话 Tab 的多分屏拓扑树管理映射 (Tab Session ID -> SplitNode)
    #[allow(dead_code)]
    pub session_split_trees: Rc<RefCell<HashMap<String, crate::terminal::SplitNode>>>,
    /// 会话 Tab 当前激活聚焦的叶子窗格 ID 映射 (Tab Session ID -> Active Pane ID)
    #[allow(dead_code)]
    pub active_pane_ids: Rc<RefCell<HashMap<String, String>>>,

}





/// 统一注册挂载所有 Slint UI 回调处理器。
///
/// 依次将窗口控制、终端会话管理、主机分组管理与开发者调试控制台的回调绑定到 AppWindow 实例。
///
/// # 参数
/// - `window`: Slint 生成的主窗口组件引用句柄
/// - `ctx`: 全局应用共享上下文对象引用
pub(crate) fn register_all_handlers(window: &AppWindow, ctx: &AppContext) {
    // 1. 挂载窗口级基础回调 (主题切换、深浅色模式、系统窗口三键操作)
    window_handlers::register_window_handlers(window, ctx);
    // 2. 挂载终端多会话与启动器回调 (Tab 切换/关闭、新建会话、键盘输入、滚轮滑动、剪贴板等)
    session_handlers::register_session_handlers(window, ctx);
    // 3. 挂载主机与分组资产回调 (分组折叠/展开、拖拽调序移动、搜索过滤、双击打开终端等)
    host_handlers::register_host_handlers(window, ctx);
    // 4. 挂载开发者调试面板专用回调 (批量造数、状态批量更新、预设写入、日志清空等)
    debug_handlers::register_debug_handlers(window, ctx);
}

