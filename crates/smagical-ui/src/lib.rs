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

/// 终端会话管理与 Slint UI 同步。
pub(crate) mod session;

/// Debug 日志面板与全局 Tracing 日志同步。
pub(crate) mod debug_ui;

/// UI 事件回调与业务路由层。
pub(crate) mod handlers;

/// 终端引擎核心层 (PTY 进程托管与 VT100 状态机)。
pub mod terminal;


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
    AppColorScheme, AppTheme, AppWindow, GroupOptionData, HostItemData, HostTreeNode,
    LocalShellItemData, LogEntryData, TabData, TerminalPaneData, TerminalSplitterData,
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

    // 启动时一次性探测本地 Shell 环境并缓存 (避免每次搜索都重新扫描文件系统)
    let cached_shells = Rc::new(local_shells::detect_local_shells());
    window.set_launcher_local_items(slint::ModelRc::from(Rc::new(slint::VecModel::from(
        (*cached_shells).clone(),
    ))));

    // 初始化核心主题服务
    let themes = match initialize_theme_service(None) {
        Ok(service) => Rc::new(service),
        Err(err) => {
            tracing::error!(target: "smagical_ui::theme", "初始化主题服务失败: {:?}", err);
            return Err(slint::PlatformError::Other("初始化主题服务失败".into()));
        }
    };

    // 应用默认初始主题 (Darcula)
    if let Err(err) = apply_theme_by_id(&window, &themes, "builtin.ui.darcula") {
        tracing::error!(target: "smagical_ui::theme", "应用默认主题失败: {:?}", err);
    }
    window.set_current_theme_name("Darcula".into());
    window.set_is_dark_mode(true);

    // 同步初始化 Debug 日志缓冲区至 Slint 界面
    sync_ui_debug_logs(&window);

    // 初始化 CoreState 核心状态引擎 (基于 MockStorage 预设种子存储)
    let core_state = Rc::new(CoreState::new_mock());

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
        initial_cards,
    ))));

    let active_sessions = Rc::new(RefCell::new(Vec::new()));
    let next_session_num = Rc::new(RefCell::new(1));
    let active_terminals = Rc::new(RefCell::new(std::collections::HashMap::new()));
    let terminal_renderer = Rc::new(RefCell::new(terminal::TerminalRenderer::new(14.0).ok()));
    let session_split_trees = Rc::new(RefCell::new(std::collections::HashMap::new()));
    let active_pane_ids = Rc::new(RefCell::new(std::collections::HashMap::new()));

    // 构造全局应用上下文
    let ctx = AppContext {
        core_state,
        master_tree,
        master_cards,
        expanded_groups,
        selector_expanded_groups,
        search_query,
        active_sessions,
        active_terminals: Rc::clone(&active_terminals),
        next_session_num,
        cached_shells,
        themes,
        terminal_renderer: Rc::clone(&terminal_renderer),
        session_split_trees: Rc::clone(&session_split_trees),
        active_pane_ids: Rc::clone(&active_pane_ids),
    };


    // 统一挂载所有区域的回调事件处理器
    register_all_handlers(&window, &ctx);

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
    let session_split_trees_timer = Rc::clone(&session_split_trees);
    let active_pane_ids_timer = Rc::clone(&active_pane_ids);
    let mut last_rendered_session = String::new();
    let mut primary_buffer: Option<slint::SharedPixelBuffer<slint::Rgba8Pixel>> = None;
    let mut pane_pixel_buffers: std::collections::HashMap<String, slint::SharedPixelBuffer<slint::Rgba8Pixel>> = std::collections::HashMap::new();
    let mut pane_rendered_images: std::collections::HashMap<String, slint::Image> = std::collections::HashMap::new();

    render_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(8),
        move || {
            if let Some(w) = window_weak.upgrade() {

                let active_id = w.get_active_session_tab().to_string();
                if active_id.is_empty() {
                    return;
                }

                let is_split = w.get_is_split();
                let ui_cols = w.get_terminal_cols() as u16;
                let ui_rows = w.get_terminal_rows() as u16;

                let mut terminals = active_terminals_timer.borrow_mut();

                if !is_split {
                    // 1. 单屏模式：泵送并渲染主视口
                    if let Some(instance) = terminals.get_mut(&active_id) {
                        if ui_cols >= 10 && ui_rows >= 5 && (instance.size.cols != ui_cols || instance.size.rows != ui_rows) {
                            let _ = instance.resize(ui_cols, ui_rows);
                        }

                        let has_new_output = instance.poll_output();
                        let switched_session = active_id != last_rendered_session;
                        if has_new_output || switched_session || instance.parser.take_dirty() {
                            let mut renderer_opt = terminal_renderer_timer.borrow_mut();
                            if let Some(renderer) = renderer_opt.as_mut() {
                                let (cw, ch) = renderer.cell_size();
                                let (cols, rows) = (instance.size.cols as u32, instance.size.rows as u32);
                                let img_w = (cols * cw + renderer.padding_x * 2).max(100);
                                let img_h = (rows * ch + renderer.padding_y * 2).max(100);

                                let mut buf = match primary_buffer.take() {
                                    Some(b) if b.width() == img_w && b.height() == img_h => b,
                                    _ => slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(img_w, img_h),
                                };

                                renderer.render_to_buffer(instance.parser.term(), instance.parser.selection(), &mut buf);
                                let image = slint::Image::from_rgba8(buf.clone());
                                w.set_terminal_screen_image(image);
                                primary_buffer = Some(buf);
                            }
                        }

                        let (hist_size, scroll_off) = instance.scroll_info();
                        w.set_terminal_history_size(hist_size as i32);
                        w.set_terminal_scroll_offset(scroll_off as i32);
                    }
                } else {
                    // 2. 任意层级嵌套二叉多分屏模式：数据驱动推导几何并渲染全量活跃叶子窗格
                    let trees = session_split_trees_timer.borrow();
                    if let Some(tree) = trees.get(&active_id) {
                        let vp_w = w.get_terminal_canvas_width().max(200.0);
                        let vp_h = w.get_terminal_canvas_height().max(100.0);
                        let (panes_layout, splitters_layout) = tree.compute_pixel_layout(vp_w, vp_h, 2.0);


                        let active_pane_id = active_pane_ids_timer

                            .borrow()
                            .get(&active_id)
                            .cloned()
                            .unwrap_or_else(|| active_id.clone());

                        let mut panes_data = Vec::with_capacity(panes_layout.len());
                        let mut renderer_opt = terminal_renderer_timer.borrow_mut();

                        for pl in &panes_layout {
                            let mut pane_image = slint::Image::default();

                            if let Some(instance) = terminals.get_mut(&pl.pane_id)
                                && let Some(renderer) = renderer_opt.as_mut()
                            {
                                let (cw, ch) = renderer.cell_size();
                                let title_bar_h = 22.0f32;
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


                            panes_data.push(TerminalPaneData {
                                pane_id: pl.pane_id.clone().into(),
                                title: pl.title.clone().into(),
                                x: pl.x,
                                y: pl.y,
                                width: pl.width,
                                height: pl.height,
                                image: pane_image,
                                is_active: pl.pane_id == active_pane_id,
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



                last_rendered_session = active_id;
            }
        },
    );



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


