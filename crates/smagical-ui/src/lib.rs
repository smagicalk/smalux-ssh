//! smagicalssh UI crate。
//!
//! 基于 Slint 声明式 GUI 框架与 `smagical-core` 核心业务层构建的现代化跨平台桌面 SSH 终端客户端。
//! 负责装配桌面应用、管理主题配色系统、调度多会话状态与生命周期。

#![deny(missing_docs)]

/// 本地终端环境探测模块。
pub mod local_shells;

/// Slint 主题资源注册、内置预设和运行时应用接口。
pub mod theme;

/// 主机资产树形数据模型与纯函数操作层。
pub(crate) mod tree_model;

/// 代码片段树形数据模型与纯函数操作层。
pub(crate) mod snippet_tree_model;

/// 终端会话管理与 Slint UI 同步。
pub(crate) mod session;

/// Debug 日志面板与全局 Tracing 日志同步。
pub(crate) mod debug_ui;

/// UI 事件回调与业务路由层。
pub(crate) mod handlers;

/// 终端引擎核心层 (PTY 进程托管与 VT100 状态机)。
pub mod terminal;

/// 快速新建终端启动器后台异步预热服务模块。
pub(crate) mod launcher_prewarm;

/// 侧边栏动态注册与 UI 同步服务。
pub(crate) mod activity_bar_service;
/// 右侧辅助抽屉动态注册与 UI 同步服务。
pub(crate) mod right_panel_service;
/// 全局气泡通知服务。
pub mod notification_service;
/// 网络隧道与出网代理全局后台常驻守护服务模块。
pub(crate) mod tunnel_daemon;


use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use slint::ComponentHandle;
use smagical_core::CoreState;
use theme::{apply_theme_by_id, initialize_theme_service};

use debug_ui::sync_ui_debug_logs;
use handlers::{register_all_handlers, AppContext};
use tree_model::{
    build_group_options, build_raw_tree_from_storage, build_visible_tree_nodes,
    calculate_max_tree_width,
};

#[allow(missing_docs, dead_code)]
mod generated {
    slint::include_modules!();
}

pub use generated::{
    ActivityBarItemData, AppColorScheme, AppTheme, AppWindow, GroupOptionData, HostItemData,
    HostTreeNode, LocalShellItemData, LogEntryData, TabData, TerminalPaneData, TerminalSplitterData,
    ToastItemData,
};



/// 创建并运行桌面应用主窗口。
///
/// 完成 Tracing 诊断日志初始化、Slint 窗口创建、本地 Shell 环境探测、主题服务加载、
/// 存储层数据模型初始化与全局回调挂载，并启动 Slint 原生事件调度主循环。
///
/// # 错误
/// 若窗口初始化失败或 Slint 平台运行时发生严重故障，将返回 `slint::PlatformError`。
pub fn run() -> Result<(), slint::PlatformError> {

    // 初始化全局 tracing 日志持久化与内存环形缓冲
    let _tracing_guard = smagical_debug::init_tracing("smalux", None);

    let window = AppWindow::new()?;

    // 启动时使用 0 磁盘 I/O 预设快速 Shell 列表初始化 UI，首帧 0ms 瞬间渲染
    let cached_shells = std::sync::Arc::new(std::sync::RwLock::new(local_shells::fast_default_shells()));
    window.set_launcher_local_items(slint::ModelRc::from(Rc::new(slint::VecModel::from(
        cached_shells.read().unwrap().clone(),
    ))));

    // 初始化核心主题仓储与服务 (接入数据层内存仓储，0 本地物理文件 I/O)
    let memory_repo = smagical_core::theme::MemoryThemeRepository::new();
    let themes = match initialize_theme_service(Some(&memory_repo)) {
        Ok(service) => Rc::new(RefCell::new(service)),
        Err(err) => {
            tracing::error!(target: "smagical_ui::theme", "初始化主题服务失败: {:?}", err);
            return Err(slint::PlatformError::Other("初始化主题服务失败".into()));
        }
    };
    let theme_repo = Some(Rc::new(RefCell::new(memory_repo)));

    // 应用默认初始主题 (Darcula)
    if let Err(err) = apply_theme_by_id(&window, &*themes.borrow(), "builtin.ui.darcula") {
        tracing::error!(target: "smagical_ui::theme", "应用默认主题失败: {:?}", err);
    }
    window.set_current_theme_id("builtin.ui.darcula".into());
    window.set_current_theme_name("Darcula".into());
    window.set_is_dark_mode(true);

    // 同步初始化全部可用主题至 Slint 界面
    crate::theme::sync_ui_themes(&window, &themes.borrow());

    // 初始化图形渲染管线标识
    let initial_pipeline = std::env::var("SLINT_BACKEND").unwrap_or_else(|_| "winit-skia".to_string());
    window.set_active_rendering_pipeline(initial_pipeline.into());

    // 同步初始化 Debug 日志缓冲区至 Slint 界面
    sync_ui_debug_logs(&window);


    // 初始化 CoreState 核心状态引擎 (基于 MockStorage 预设种子存储)
    let core_state = Rc::new(CoreState::new_mock());

    // 同步 Debug 开启状态与侧边栏动态注册菜单项到 Slint 界面
    let is_dbg = smagical_debug::is_debug_enabled();
    window.set_is_debug_enabled(is_dbg);
    core_state.activity_bar().set_visible("debug", is_dbg);
    activity_bar_service::sync_activity_bar_ui(&window, &core_state);
    right_panel_service::sync_right_panel_ui(&window, &core_state);




    // 启动本地终端异步探测服务 (0ms 阻塞主线程)
    local_shells::start_local_shell_discovery(
        std::sync::Arc::clone(&cached_shells),
        window.as_weak(),
    );

    // 注册启动器资产数据后台异步预热服务
    let prewarm_service = std::sync::Arc::new(launcher_prewarm::LauncherPrewarmService::new(
        core_state.storage().clone(),
        window.as_weak(),
    ));
    prewarm_service.register(core_state.event_manager());

    // 注册网络隧道与代理全局后台常驻守护服务 (处理跟随整个应用的全局自启与退出注销)
    let tunnel_daemon = std::sync::Arc::new(tunnel_daemon::TunnelDaemonService::new(
        core_state.storage().clone(),
        window.as_weak(),
    ));
    tunnel_daemon.register(core_state.event_manager());

    // 触发全局应用启动事件
    core_state.events().dispatch(&smagical_core::AppBootEvent);



    // 从存储层读取初始主控树形结构与分组生成器
    let initial_tree = build_raw_tree_from_storage(core_state.storage().as_ref());
    let master_tree = Rc::new(RefCell::new(initial_tree));

    // 从存储层初始化树形结构折叠状态 (读取所有 is_expanded == true 的分组)
    let initial_expanded: HashSet<String> = core_state
        .storage()
        .groups()
        .list_all()
        .unwrap_or_default()
        .into_iter()
        .filter(|g| g.is_expanded)
        .map(|g| g.id)
        .collect();
    let expanded_groups = Rc::new(RefCell::new(initial_expanded));

    let search_query = Rc::new(RefCell::new(String::new()));

    // 动态初始化上级分组选择器展开状态：从存储中读取所有顶级分组（parent_id 为 None）
    let mut initial_selector_expanded = HashSet::from(["root".to_string()]);
    core_state
        .storage()
        .groups()
        .list_all()
        .unwrap_or_default()
        .into_iter()
        .filter(|g| g.parent_id.is_none())
        .for_each(|g| {
            initial_selector_expanded.insert(g.id);
        });
    let selector_expanded_groups = Rc::new(RefCell::new(initial_selector_expanded));

    // 初始渲染上级分组选项数据
    let initial_options =
        build_group_options(&master_tree.borrow(), &selector_expanded_groups.borrow());
    window.set_group_options(slint::ModelRc::from(Rc::new(slint::VecModel::from(
        initial_options,
    ))));

    // 初始渲染树形节点
    let initial_nodes =
        build_visible_tree_nodes(&master_tree.borrow(), &expanded_groups.borrow());
    window.set_tree_content_width(calculate_max_tree_width(&initial_nodes));
    window.set_tree_nodes(slint::ModelRc::from(Rc::new(slint::VecModel::from(
        initial_nodes,
    ))));

    // 从存储层初始渲染卡片列表
    let all_hosts = core_state.storage().hosts().list_all().unwrap_or_default();
    let all_groups = core_state.storage().groups().list_all().unwrap_or_default();
    let initial_cards: Vec<HostItemData> = all_hosts
        .into_iter()
        .map(|h| {
            let group_name = h
                .parent_group_id
                .as_deref()
                .and_then(|p_id| all_groups.iter().find(|g| g.id == p_id).map(|g| g.name.clone()))
                .unwrap_or_else(|| "未分组".to_string());
            HostItemData {
                id: h.id.into(),
                name: h.name.into(),
                address: h.address.into(),
                port: h.port as i32,
                group: group_name.into(),
                status: h.status.to_string().into(),
                ping_ms: h.ping_ms,
            }
        })
        .collect();
    let master_cards = Rc::new(RefCell::new(initial_cards.clone()));
    window.set_hosts(slint::ModelRc::from(Rc::new(slint::VecModel::from(
        initial_cards.clone(),
    ))));
    window.set_launcher_host_items(slint::ModelRc::from(Rc::new(slint::VecModel::from(
        initial_cards,
    ))));


    let next_session_num = Rc::new(RefCell::new(1));
    let active_terminals = Rc::new(RefCell::new(std::collections::HashMap::new()));
    let terminal_renderer = Rc::new(RefCell::new(terminal::TerminalRenderer::new(14.0).ok()));
    let pane_groups = Rc::new(RefCell::new(Vec::new()));
    let global_split_tree = Rc::new(RefCell::new(None));
    let active_pane_id = Rc::new(RefCell::new(String::new()));
    let zoomed_pane_id = Rc::new(RefCell::new(None));
    let next_pane_num = Rc::new(RefCell::new(1));

    let collapsed_history_groups = Rc::new(RefCell::new(HashSet::new()));
    let history_view_mode = Rc::new(RefCell::new("timeline".to_string()));
    let history_search_query = Rc::new(RefCell::new(String::new()));

    let home_path = directories::BaseDirs::new()
        .map(|p| p.home_dir().to_string_lossy().to_string())
        .unwrap_or_else(|| "/".to_string());
    let initial_local_files = smagical_core::scan_local_directory(&std::path::PathBuf::from(&home_path)).unwrap_or_default();

    // 默认左侧有 1 个本地主目录 Tab，右侧默认无 Tab (呈现连接提示)
    let local_tabs = Rc::new(RefCell::new(vec![
        smagical_core::LocalFileTabSession::new(
            "ltab-1",
            "本地 (主目录)",
            &home_path,
        )
    ]));
    let active_local_tab_id = Rc::new(RefCell::new("ltab-1".to_string()));
    let remote_tabs = Rc::new(RefCell::new(Vec::new()));
    let active_remote_tab_id = Rc::new(RefCell::new(String::new()));
    let local_current_path = Rc::new(RefCell::new(home_path));
    let remote_current_path = Rc::new(RefCell::new(String::new()));
    let local_file_nodes = Rc::new(RefCell::new(initial_local_files));
    let remote_file_nodes = Rc::new(RefCell::new(Vec::new()));
    let transfer_tasks = Rc::new(RefCell::new(Vec::<smagical_core::TransferTask>::new()));
    let notifications = notification_service::NotificationManager::new(window.as_weak());

    // 代码片段树形与多层层级初始状态
    let initial_snippet_master = snippet_tree_model::build_raw_snippet_tree_from_storage(core_state.storage().as_ref());
    let mut initial_snippet_expanded = HashSet::new();
    core_state.storage().snippets().list_groups().unwrap_or_default()
        .into_iter()
        .for_each(|g| {
            if g.is_expanded {
                initial_snippet_expanded.insert(g.id);
            }
        });
    let master_snippet_tree = Rc::new(RefCell::new(initial_snippet_master));
    let expanded_snippet_groups = Rc::new(RefCell::new(initial_snippet_expanded));
    let snippet_search_query = Rc::new(RefCell::new(String::new()));

    let tunnel_search_query = Rc::new(RefCell::new(String::new()));
    let tunnel_filter_category = Rc::new(RefCell::new("all".to_string()));

    let initial_config = core_state.storage().config().get().unwrap_or_default();

    let wallpapers = Rc::new(RefCell::new(initial_config.wallpaper_list.clone()));
    let active_wallpaper_idx = Rc::new(RefCell::new(initial_config.wallpaper_active_index));
    let wallpaper_timer = Rc::new(RefCell::new(None));
    let wallpaper_preload_timer = Rc::new(RefCell::new(None));
    let wallpaper_cache = Rc::new(RefCell::new(std::collections::HashMap::new()));

    // 构造全局应用上下文
    let ctx = AppContext {
        core_state: Rc::clone(&core_state),
        master_tree,

        master_cards,
        expanded_groups,
        selector_expanded_groups,
        search_query,
        active_terminals: Rc::clone(&active_terminals),
        next_session_num,
        cached_shells,
        themes,
        theme_repo,
        wallpapers,
        active_wallpaper_idx,
        wallpaper_timer,
        wallpaper_cache,
        wallpaper_preload_timer,
        terminal_renderer: Rc::clone(&terminal_renderer),

        pane_groups: Rc::clone(&pane_groups),
        global_split_tree: Rc::clone(&global_split_tree),
        active_pane_id: Rc::clone(&active_pane_id),
        zoomed_pane_id: Rc::clone(&zoomed_pane_id),
        next_pane_num: Rc::clone(&next_pane_num),

        collapsed_history_groups,
        history_view_mode,
        history_search_query,
        persistence_guard: std::sync::Arc::new(crate::session::SessionPersistenceGuard::default()),

        local_tabs: Rc::clone(&local_tabs),
        active_local_tab_id: Rc::clone(&active_local_tab_id),
        remote_tabs: Rc::clone(&remote_tabs),
        active_remote_tab_id: Rc::clone(&active_remote_tab_id),
        local_current_path: Rc::clone(&local_current_path),
        remote_current_path: Rc::clone(&remote_current_path),
        local_file_nodes: Rc::clone(&local_file_nodes),
        remote_file_nodes: Rc::clone(&remote_file_nodes),
        transfer_tasks: Rc::clone(&transfer_tasks),
        notifications,

        master_snippet_tree,
        expanded_snippet_groups,
        snippet_search_query,

        tunnel_search_query,
        tunnel_filter_category,
    };

    // 初始同步历史会话抽屉、双盘文件浏览器、代码片段中心与网络隧道中枢数据
    handlers::history_handlers::sync_ui_history(&window, &ctx);
    handlers::file_handlers::sync_file_explorer_ui(&window, &ctx);
    handlers::snippet_handlers::sync_ui_snippets(&window, &ctx);
    handlers::tunnel_handlers::sync_ui_tunnels(&window, &ctx);

    // 初始化默认网络隧道选中项
    if let Ok(all_tuns) = ctx.core_state.storage().tunnels().list_all() {
        if let Some(first) = all_tuns.first() {
            window.set_active_tunnel_id(first.id.clone().into());
            window.set_tunnel_form_id(first.id.clone().into());
            window.set_tunnel_form_name(first.name.clone().into());
            window.set_tunnel_form_type(first.tunnel_type.as_str().into());
            window.set_tunnel_form_ssh_host_id(first.ssh_host_id.clone().unwrap_or_default().into());
            window.set_tunnel_form_ssh_host_name(first.ssh_host_name.clone().into());
            window.set_tunnel_form_local_bind(first.local_bind.clone().into());
            window.set_tunnel_form_local_port(first.local_port.to_string().into());
            window.set_tunnel_form_remote_host(first.remote_host.clone().into());
            window.set_tunnel_form_remote_port(first.remote_port.to_string().into());
            window.set_tunnel_form_auto_start(first.auto_start);
            window.set_tunnel_form_auto_reconnect(first.auto_reconnect);
            window.set_tunnel_form_remote_dns(first.remote_dns);
            window.set_tunnel_form_compression(first.compression);
            window.set_tunnel_form_notes(first.notes.clone().into());
        }
    }

    // 统一挂载所有区域的回调事件处理器
    register_all_handlers(&window, &ctx);

    // 同步底层配置仓储 (ConfigRepository) 状态至 Slint 界面
    window.set_current_language(initial_config.language.as_str().into());
    window.set_setting_close_action(initial_config.close_action.as_str().into());
    window.set_setting_start_on_boot(initial_config.start_on_boot);
    window.set_setting_confirm_close_tab(initial_config.confirm_close_tab);
    window.set_setting_confirm_close_active(initial_config.confirm_close_active);
    window.set_setting_copy_on_select(initial_config.copy_on_select);
    window.set_setting_paste_on_right_click(initial_config.paste_on_right_click);
    window.set_setting_warn_multiline_paste(initial_config.warn_on_multiline_paste);

    window.set_current_theme_id(initial_config.theme_id.as_str().into());
    window.set_is_dark_mode(initial_config.is_dark_mode);
    window.invoke_switch_theme(initial_config.theme_id.as_str().into());

    window.set_terminal_font_family(initial_config.font_family.as_str().into());
    window.set_terminal_font_size(initial_config.font_size);
    if let Some(ref mut r) = *terminal_renderer.borrow_mut() {
        r.set_cursor_style(&initial_config.cursor_style);
        r.set_cursor_blink(initial_config.cursor_blink);
        if let Some(bytes) = terminal::renderer::find_font_by_name(&initial_config.font_family) {
            let _ = r.update_font(&bytes, initial_config.font_size);
        }
    }

    window.set_is_debug_enabled(initial_config.debug_enabled);
    window.set_flag_desktop_notifications(initial_config.flag_desktop_notifications);
    window.set_flag_terminal_crt_shader(initial_config.flag_terminal_crt_shader);
    window.set_flag_cloud_sync(initial_config.flag_cloud_sync);
    window.set_flag_terminal_scratchpad(initial_config.flag_terminal_scratchpad);

    // 同步壁纸状态至全局 AppTheme 令牌
    let theme_global = window.global::<AppTheme>();
    theme_global.set_wallpaper_mode(initial_config.wallpaper_mode.as_str().into());
    theme_global.set_wallpaper_opacity(initial_config.wallpaper_opacity);
    theme_global.set_modal_opacity(initial_config.modal_opacity);
    window.set_setting_modal_opacity(initial_config.modal_opacity);

    // 壁纸画廊数据与初始渲染
    if !initial_config.wallpaper_list.is_empty() {
        let slint_strings: Vec<slint::SharedString> = initial_config.wallpaper_list.iter().map(|s| s.as_str().into()).collect();
        window.set_wallpaper_list(slint::ModelRc::new(slint::VecModel::from(slint_strings)));
        window.set_wallpaper_active_index(initial_config.wallpaper_active_index as i32);
        if initial_config.wallpaper_active_index < initial_config.wallpaper_list.len() {
            let wp_path = &initial_config.wallpaper_list[initial_config.wallpaper_active_index];
            window.invoke_set_wallpaper(
                initial_config.wallpaper_mode.as_str().into(),
                wp_path.as_str().into(),
                initial_config.wallpaper_opacity,
            );
        }
    }



    // 持久化多窗格与分割条数据模型引用（保持 ModelRc 实例单一持久，避免 Slint 重建 UI 组件导致拖拽焦点丢失）
    let panes_model = Rc::new(slint::VecModel::<TerminalPaneData>::default());
    let splitters_model = Rc::new(slint::VecModel::<TerminalSplitterData>::default());
    window.set_terminal_panes(slint::ModelRc::new(Rc::clone(&panes_model)));
    window.set_terminal_splitters(slint::ModelRc::new(Rc::clone(&splitters_model)));

    let panes_model_timer = Rc::clone(&panes_model);
    let splitters_model_timer = Rc::clone(&splitters_model);

    // 启动 120Hz (8ms) 超流畅终端位图渲染与 PTY 异步流输出泵送定时器
    let render_timer = slint::Timer::default();
    let window_weak = window.as_weak();
    let active_terminals_timer = Rc::clone(&active_terminals);
    let terminal_renderer_timer = Rc::clone(&terminal_renderer);
    let pane_groups_timer = Rc::clone(&pane_groups);
    let global_split_tree_timer = Rc::clone(&global_split_tree);
    let active_pane_id_timer = Rc::clone(&active_pane_id);
    let zoomed_pane_id_timer = Rc::clone(&zoomed_pane_id);
    let mut last_rendered_session = String::new();
    let mut primary_buffer: Option<slint::SharedPixelBuffer<slint::Rgba8Pixel>> = None;
    let mut pane_pixel_buffers: std::collections::HashMap<String, slint::SharedPixelBuffer<slint::Rgba8Pixel>> = std::collections::HashMap::new();
    let mut pane_rendered_images: std::collections::HashMap<String, slint::Image> = std::collections::HashMap::new();
    let mut pane_tab_models: std::collections::HashMap<String, (Vec<TabData>, slint::ModelRc<TabData>)> = std::collections::HashMap::new();

    render_timer.start(

        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(8),
        move || {
            if let Some(w) = window_weak.upgrade() {
                let groups = pane_groups_timer.borrow();
                if groups.is_empty() {
                    return;
                }

                let is_split = global_split_tree_timer.borrow().is_some();
                let mut terminals = active_terminals_timer.borrow_mut();
                let mut renderer_opt = terminal_renderer_timer.borrow_mut();

                if !is_split {
                    // 1. 单屏模式：泵送并渲染主视口
                    let main_group = &groups[0];
                    if let Some(active_sess) = main_group.get_active_session() {
                        let active_id = active_sess.session_id.clone();
                        let ui_cols = w.get_terminal_cols() as u16;
                        let ui_rows = w.get_terminal_rows() as u16;

                        if let Some(instance) = terminals.get_mut(&active_id) {
                            if instance.size.cols != ui_cols || instance.size.rows != ui_rows {
                                let _ = instance.resize(ui_cols, ui_rows);
                            }

                            let has_new_output = instance.poll_output();
                            let is_dirty = instance.parser.take_dirty();

                            if let Some(renderer) = renderer_opt.as_mut() {
                                let (cw, ch) = renderer.cell_size();
                                let img_w = (ui_cols as u32 * cw + renderer.padding_x * 2).max(100);
                                let img_h = (ui_rows as u32 * ch + renderer.padding_y * 2).max(60);

                                let mut buf = match primary_buffer.take() {
                                    Some(b) if b.width() == img_w && b.height() == img_h => b,
                                    _ => slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(img_w, img_h),
                                };

                                if has_new_output || is_dirty || last_rendered_session != active_id {
                                    renderer.render_to_buffer(instance.parser.term(), instance.parser.selection(), &mut buf);
                                    let image = slint::Image::from_rgba8(buf.clone());
                                    w.set_terminal_screen_image(image);
                                    primary_buffer = Some(buf);
                                    last_rendered_session = active_id;
                                } else {
                                    primary_buffer = Some(buf);
                                }
                            }

                            let (hist_size, scroll_off) = instance.scroll_info();
                            w.set_terminal_history_size(hist_size as i32);
                            w.set_terminal_scroll_offset(scroll_off as i32);
                        }
                    }
                } else {
                    // 2. 任意层级嵌套二叉多分屏模式：数据驱动推导几何并渲染全量活跃叶子窗格
                    let trees_guard = global_split_tree_timer.borrow();
                    if let Some(tree) = trees_guard.as_ref() {
                        let vp_w = w.get_terminal_canvas_width().max(200.0);
                        let vp_h = w.get_terminal_canvas_height().max(100.0);
                        let current_zoom = zoomed_pane_id_timer.borrow().clone();
                        let (panes_layout, splitters_layout) = tree.compute_pixel_layout(vp_w, vp_h, 2.0, current_zoom.as_deref());

                        let active_pid = active_pane_id_timer.borrow().clone();
                        let total_panes_len = panes_layout.len();
                        let mut panes_data = Vec::with_capacity(total_panes_len);

                        for (idx, pl) in panes_layout.iter().enumerate() {
                            let mut pane_image = slint::Image::default();
                            let group_opt = groups.iter().find(|g| g.pane_id == pl.pane_id);
                            let active_sess_opt = group_opt.and_then(|g| g.get_active_session());

                            if let Some(active_sess) = active_sess_opt
                                && let Some(instance) = terminals.get_mut(&active_sess.session_id)
                                && let Some(renderer) = renderer_opt.as_mut()
                            {
                                let (cw, ch) = renderer.cell_size();
                                let title_bar_h = 36.0f32;
                                let content_w = (pl.width - (renderer.padding_x * 2) as f32).max(40.0);
                                let content_h = (pl.height - title_bar_h - (renderer.padding_y * 2) as f32).max(20.0);

                                let target_cols = ((content_w / cw as f32) as u16).max(10);
                                let target_rows = ((content_h / ch as f32) as u16).max(3);

                                if instance.size.cols != target_cols || instance.size.rows != target_rows {
                                    let _ = instance.resize(target_cols, target_rows);
                                }

                                let has_new_output = instance.poll_output();
                                let is_dirty = instance.parser.take_dirty();

                                let img_w = (target_cols as u32 * cw + renderer.padding_x * 2).max(50);
                                let img_h = (target_rows as u32 * ch + renderer.padding_y * 2).max(30);

                                let mut buf = match pane_pixel_buffers.remove(&pl.pane_id) {
                                    Some(b) if b.width() == img_w && b.height() == img_h => b,
                                    _ => slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(img_w, img_h),
                                };

                                if has_new_output || is_dirty || !pane_rendered_images.contains_key(&pl.pane_id) {
                                    renderer.render_to_buffer(instance.parser.term(), instance.parser.selection(), &mut buf);
                                    let img = slint::Image::from_rgba8(buf.clone());
                                    pane_rendered_images.insert(pl.pane_id.clone(), img.clone());
                                    pane_image = img;
                                } else if let Some(cached_img) = pane_rendered_images.get(&pl.pane_id) {
                                    pane_image = cached_img.clone();
                                }

                                pane_pixel_buffers.insert(pl.pane_id.clone(), buf);
                            }

                            let (title, status, pane_tabs, active_tab_id) = if let Some(group) = group_opt {
                                let act_id = group.active_tab_id.clone();
                                let act_title = group.get_active_session().map(|s| s.display_title.clone()).unwrap_or_else(|| pl.title.clone());
                                let act_status = group.get_active_session().map(|s| s.host_status.clone()).unwrap_or_else(|| "online".to_string());
                                (act_title, act_status, group.to_tab_data_list(), act_id)
                            } else {
                                (pl.title.clone(), "online".to_string(), Vec::new(), String::new())
                            };

                            let pane_tabs_rc = match pane_tab_models.get_mut(&pl.pane_id) {
                                Some((cached_tabs, cached_rc)) if *cached_tabs == pane_tabs => cached_rc.clone(),
                                _ => {
                                    let rc = slint::ModelRc::from(std::rc::Rc::new(slint::VecModel::from(pane_tabs.clone())));
                                    pane_tab_models.insert(pl.pane_id.clone(), (pane_tabs, rc.clone()));
                                    rc
                                }
                            };

                            panes_data.push(TerminalPaneData {
                                pane_id: pl.pane_id.clone().into(),
                                title: title.into(),
                                x: pl.x,
                                y: pl.y,
                                width: pl.width,
                                height: pl.height,
                                image: pane_image,
                                is_active: pl.pane_id == active_pid,
                                pane_index: (idx + 1) as i32,
                                total_panes: total_panes_len as i32,
                                is_zoomed: current_zoom.as_deref() == Some(&pl.pane_id),
                                status: status.into(),
                                tabs: pane_tabs_rc,
                                active_tab_id: active_tab_id.into(),
                            });
                        }


                        update_model_in_place(&panes_model_timer, panes_data);

                        let splitters_data: Vec<TerminalSplitterData> = splitters_layout
                            .into_iter()
                            .map(|sl| TerminalSplitterData {
                                splitter_id: sl.splitter_id.into(),
                                is_vertical: sl.is_vertical,
                                x: sl.x,
                                y: sl.y,
                                width: sl.width,
                                height: sl.height,
                            })
                            .collect();
                        update_model_in_place(&splitters_model_timer, splitters_data);
                    }
                }
            }
        },
    );




    // 触发全局应用界面首帧就绪事件
    core_state.events().dispatch(&smagical_core::AppReadyEvent);

    window.run()?;
    Ok(())

}

/// 智能就地更新 Slint 动态数据模型（仅在行数据发生变化时更新行，杜绝全量 reset 导致 UI 组件重构与鼠标拖拽焦点丢失）。
fn update_model_in_place<T: PartialEq + Clone + 'static>(model: &slint::VecModel<T>, new_items: Vec<T>) {
    use slint::Model;
    if model.row_count() == new_items.len() {
        for (i, item) in new_items.into_iter().enumerate() {
            if let Some(existing) = model.row_data(i) {
                if existing != item {
                    model.set_row_data(i, item);
                }
            } else {
                model.set_row_data(i, item);
            }
        }
    } else {
        model.set_vec(new_items);
    }
}


