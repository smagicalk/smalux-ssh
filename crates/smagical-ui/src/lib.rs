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

/// 代码片段智能参数记忆、使用度量追踪与物理音效反馈服务。
pub mod snippet_service;

/// 终端会话管理与 Slint UI 同步。
pub(crate) mod session;

/// Debug 日志面板与全局 Tracing 日志同步。
pub(crate) mod debug_ui;

/// UI 事件回调与业务路由层。
pub(crate) mod handlers;

/// 集中式状态管理与增量渲染中枢。
pub(crate) mod store;

/// 开发者调试控制面板、场景预设与 Tracing 全局日志模块。
pub mod debug;
pub use debug::*;

/// 终端引擎核心层 (PTY 进程托管与 VT100 状态机)。
pub mod terminal;

/// 快速新建终端启动器后台异步预热服务模块。
pub(crate) mod launcher_prewarm;

/// 侧边栏动态注册与 UI 同步服务。
pub(crate) mod activity_bar_service;
/// 右侧辅助抽屉动态注册与 UI 同步服务。
pub(crate) mod right_panel_service;
/// 统一视图生命周期管理与内存按需卸载调度中枢。
pub(crate) mod view_lifecycle;
/// 全局气泡通知服务。
pub mod notification_service;
/// 网络隧道与出网代理全局后台常驻守护服务模块。
pub(crate) mod tunnel_daemon;
/// 容灾备份与多端快照同步全局后台常驻守护服务模块。
pub mod backup_daemon;
/// 图形渲染管线本地持久化配置与启动分发模块。
pub mod pipeline_config;
/// 桌面系统托盘常驻守护与交互服务模块。
pub mod tray;
/// 存储后端模式本地持久化配置模块。
pub mod storage_config;
/// smagical-ui 公共工具体系 (线程派发、零拷贝转换、极速匹配与集合模型)。
pub mod common;
/// 异步运行时与同步阻塞调度工具。
pub mod async_util;
pub use async_util::{block_on, spawn_async};
/// SSH 密钥对现场生成器。
pub mod keygen;
/// 第三方终端资产迁移与格式导入解析器。
pub mod importer;
/// 远程 SFTP 与 SSH 文件系统操作模块。
pub mod sftp;
/// 远程 Linux 系统性能监控指标探针引擎。
pub mod monitor;
/// 会话操作安全审计日志服务。
pub mod audit_logger;
/// 高性能并发文件传输队列调度管理中枢。
pub mod transfer_manager;


use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use crate::common::to_model_rc;

use slint::ComponentHandle;
use smagical_core::CoreState;
use theme::{apply_theme_by_id, initialize_theme_service};

use debug_ui::sync_ui_debug_logs;
use handlers::{register_all_handlers, AppContext};
use session::sync_active_session_ui;
use tree_model::{
    build_cards_from_records, build_group_options, build_raw_tree, build_visible_tree_nodes,
    calculate_max_tree_width,
};

#[doc(hidden)]
#[allow(missing_docs, dead_code)]
pub use smagical_ui_kernel as generated;

pub use smagical_ui_kernel::*;

/// 系统托盘图标 PNG 静态字节数据
pub static TRAY_PNG_BYTES: &[u8] = include_bytes!("../../ui/common/ui/assets/tray-icon.png");

/// 终端渲染默认字体 JetBrains Mono 静态字节数据
pub static JETBRAINS_MONO_BYTES: &[u8] = include_bytes!("../../ui/common/ui/assets/fonts/JetBrainsMono-Regular.ttf");



/// 创建并运行桌面应用主窗口。
///
/// 完成 Tracing 诊断日志初始化、Slint 窗口创建、本地 Shell 环境探测、主题服务加载、
/// 存储层数据模型初始化与全局回调挂载，并启动 Slint 原生事件调度主循环。
///
/// # 错误
/// 若窗口初始化失败或 Slint 平台运行时发生严重故障，将返回 `slint::PlatformError`。
pub fn run() -> Result<(), slint::PlatformError> {

    // 初始化全局 tracing 日志持久化与内存环形缓冲
    let _tracing_guard = crate::debug::init_tracing("smalux", None);

    let window = AppWindow::new()?;

    // 启动时使用 0 磁盘 I/O 预设快速 Shell 列表初始化 UI，首帧 0ms 瞬间渲染
    let cached_shells = std::sync::Arc::new(std::sync::RwLock::new(local_shells::fast_default_shells()));
    window.global::<WindowBridge>().set_launcher_local_items(to_model_rc(
        cached_shells.read().unwrap().clone(),
    ));

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
    window.global::<WindowBridge>().set_current_theme_id("builtin.ui.darcula".into());
    window.global::<WindowBridge>().set_current_theme_name("Darcula".into());
    window.global::<WindowBridge>().set_is_dark_mode(true);

    // 同步初始化全部可用主题至 Slint 界面
    crate::theme::sync_ui_themes(&window, &themes.borrow());

    // 初始化图形渲染管线标识 (从磁盘持久化配置或环境变量载入)
    let initial_pipeline = pipeline_config::init_runtime_pipeline();
    tracing::info!(target: "smagical_ui::render", "当前加载生效的图形渲染管线: [{}]", initial_pipeline);
    window.global::<WindowBridge>().set_active_rendering_pipeline(initial_pipeline.clone().into());
    window.global::<SettingsBridge>().set_active_rendering_pipeline(initial_pipeline.into());

    // 同步初始化 Debug 日志缓冲区至 Slint 界面
    sync_ui_debug_logs(&window);


    // 根据本地持久化配置加载存储模式 (未配置或默认启用物理持久化 SQLite 引擎)
    let is_mock_pref = storage_config::get_persisted_storage_mode()
        .map(|m| m == "mock")
        .unwrap_or(false);

    let (storage, is_mock): (std::sync::Arc<dyn smagical_core::storage::AppStorage>, bool) = if is_mock_pref {
        tracing::info!(target: "smagical_ui::storage", "依据本地持久化配置加载: [MockStorage] 内存种子存储");
        (std::sync::Arc::new(smagical_storage::MockStorage::new_seeded()), true)
    } else {
        match async_util::block_on(smagical_storage::SeaOrmStorage::open_default()) {
            Ok(s) => {
                tracing::info!(target: "smagical_ui::storage", "成功初始化物理持久化数据库: [SeaOrmStorage] SQLite 引擎");
                (std::sync::Arc::new(s), false)
            }
            Err(e) => {
                tracing::error!(target: "smagical_ui::storage", "打开物理 SQLite 数据库失败: {:?}，自动回退至内存种子存储", e);
                (std::sync::Arc::new(smagical_storage::MockStorage::new_seeded()), true)
            }
        }
    };
    let core_state = Rc::new(CoreState::with_storage(storage, is_mock));
    let session_driver = std::sync::Arc::new(smagical_ssh::RusshSessionDriver::new());
    let sftp_driver = std::sync::Arc::new(smagical_ssh::RusshSftpDriver::new());
    let native_sftp_driver = Some(std::sync::Arc::clone(&sftp_driver));
    let tunnel_driver = std::sync::Arc::new(smagical_ssh::RusshTunnelDriver::new());
    let keygen_service = std::sync::Arc::new(smagical_ssh::NativeKeygenService::new());
    let metrics_driver = std::sync::Arc::new(smagical_ssh::RusshMetricsDriver::with_ssh_service(
        std::sync::Arc::clone(&session_driver) as _,
    ));

    core_state.set_ssh_service(session_driver);
    core_state.set_sftp_service(sftp_driver);
    core_state.set_tunnel_service(tunnel_driver);
    core_state.set_keygen_service(keygen_service);
    core_state.set_metrics_service(metrics_driver);
    window.global::<DebugBridge>().set_use_mock_storage(is_mock);
    let sb = window.global::<SettingsBridge>();
    sb.set_setting_storage_mode(if is_mock { "mock".into() } else { "physical".into() });
    let db_path = smagical_storage::seaorm::get_default_sqlite_path();
    sb.set_setting_storage_path(db_path.to_string_lossy().to_string().into());

    // -------------------------------------------------------------------------
    // 冷启动数据统一异步并发加载 (0 阻塞，全量 I/O 并发拉取)
    // -------------------------------------------------------------------------
    let app_storage = core_state.storage();
    let (
        config_res,
        groups_res,
        hosts_res,
        credentials_res,
        snippet_groups_res,
        snippets_res,
    ) = async_util::block_on(async {
        tokio::join!(
            app_storage.config().get(),
            app_storage.groups().list_all(),
            app_storage.hosts().list_all(),
            app_storage.credentials().list_all(),
            app_storage.snippets().list_groups(),
            app_storage.snippets().list_all(),
        )
    });

    let initial_config = config_res.unwrap_or_default();
    let all_groups = groups_res.unwrap_or_default();
    let all_hosts = hosts_res.unwrap_or_default();
    let all_credentials = credentials_res.unwrap_or_default();
    let all_snippet_groups = snippet_groups_res.unwrap_or_default();
    let all_snippets = snippets_res.unwrap_or_default();

    // -------------------------------------------------------------------------
    // 国际化语言环境初始化 (根据持久化配置生效 Slint 捆绑翻译与 UI 语言)
    // -------------------------------------------------------------------------
    let initial_lang = initial_config.language.as_str();
    let slint_lang_code = match initial_lang {
        "en-US" | "en" => "en",
        _ => "",
    };
    if !slint_lang_code.is_empty() {
        if let Err(err) = slint::select_bundled_translation(slint_lang_code) {
            tracing::error!(target: "smagical_ui::i18n", "初始化加载捆绑翻译失败: {:?}", err);
        } else {
            tracing::info!(target: "smagical_ui::i18n", "冷启动成功生效 UI 语言: [{}]", slint_lang_code);
        }
    }

    let wb = window.global::<WindowBridge>();
    wb.set_current_language(initial_config.language.as_str().into());
    window.global::<SettingsBridge>().set_setting_language(initial_config.language.as_str().into());

    // 同步 Debug 开启状态与侧边栏动态注册菜单项到 Slint 界面
    let is_dbg = crate::debug::is_debug_enabled();
    wb.set_is_debug_enabled(is_dbg);
    core_state.activity_bar().set_visible("debug", is_dbg);
    activity_bar_service::sync_activity_bar_ui(&window, &core_state);
    right_panel_service::sync_right_panel_ui(&window, &core_state);




    // 注册本地终端异步探测服务 (跟随 AppBootEvent 引导生命周期自启)
    let shell_discovery = std::sync::Arc::new(local_shells::LocalShellDiscoveryService::new(
        std::sync::Arc::clone(&cached_shells),
        window.as_weak(),
    ));
    shell_discovery.register(core_state.event_manager());

    // 注册启动器资产数据后台异步预热服务 (跟随 AppReadyEvent 首帧就绪)
    let prewarm_service = std::sync::Arc::new(launcher_prewarm::LauncherPrewarmService::new(
        core_state.storage().clone(),
        window.as_weak(),
    ));
    prewarm_service.register(core_state.event_manager());

    // 注册网络隧道与代理全局后台常驻守护服务 (跟随 AppReadyEvent 首帧就绪与 AppBeforeExitEvent 退出注销)
    let tunnel_daemon = std::sync::Arc::new(tunnel_daemon::TunnelDaemonService::new(
        core_state.storage().clone(),
        core_state.tunnels().clone(),
        window.as_weak(),
    ));
    tunnel_daemon.register(core_state.event_manager());

    // 触发全局应用引导启动事件 (触发 Shell 探测等引导期后台服务)
    core_state.events().dispatch(&smagical_core::AppBootEvent);

    // 初始化并启动系统托盘常驻守护服务 (支持最小化到托盘、右键菜单快捷操作与呼出唤醒)
    let _tray_service = match tray::TrayService::init(&window) {
        Ok(t) => {
            tracing::info!(target: "smalux::tray", "桌面系统托盘服务挂载就绪");
            Some(t)
        }
        Err(err) => {
            tracing::warn!(target: "smalux::tray", "初始化系统托盘服务受限: {:?}", err);
            None
        }
    };



    // 从存储层读取初始主控树形结构与分组生成器 (利用冷启动并发缓存数据纯内存快速构建)
    let initial_tree = build_raw_tree(&all_groups, &all_hosts, &all_credentials);
    let master_tree = Arc::new(RwLock::new(initial_tree));

    // 从存储层初始化树形结构折叠状态 (读取所有 is_expanded == true 的分组)
    let initial_expanded: HashSet<String> = all_groups
        .iter()
        .filter(|g| g.is_expanded)
        .map(|g| g.id.clone())
        .collect();
    let expanded_groups = Arc::new(RwLock::new(initial_expanded));

    let search_query = Arc::new(RwLock::new(String::new()));

    // 动态初始化上级分组选择器展开状态：从存储中读取所有顶级分组（parent_id 为 None）
    let mut initial_selector_expanded = HashSet::from(["root".to_string()]);
    all_groups
        .iter()
        .filter(|g| g.parent_id.is_none())
        .for_each(|g| {
            initial_selector_expanded.insert(g.id.clone());
        });
    let selector_expanded_groups = Arc::new(RwLock::new(initial_selector_expanded));

    // 初始渲染上级分组选项数据
    let hb = window.global::<HostsBridge>();
    let initial_options =
        build_group_options(&master_tree.read().unwrap(), &selector_expanded_groups.read().unwrap());
    hb.set_group_options(to_model_rc(initial_options));

    // 初始渲染树形节点
    let initial_nodes =
        build_visible_tree_nodes(&master_tree.read().unwrap(), &expanded_groups.read().unwrap());
    hb.set_tree_content_width(calculate_max_tree_width(&initial_nodes));
    hb.set_tree_nodes(to_model_rc(initial_nodes));

    // 从冷启动并发缓存数据初始渲染卡片列表 (纯内存 0 I/O)
    let initial_cards = build_cards_from_records(&all_hosts, &all_groups);
    let master_cards = Arc::new(RwLock::new(initial_cards.clone()));
    hb.set_hosts(to_model_rc(initial_cards.clone()));
    window.global::<WindowBridge>().set_launcher_host_items(to_model_rc(initial_cards));


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
    notifications.set_duration_preset(&initial_config.toast_duration);

    // 代码片段树形与多层层级初始状态 (利用冷启动并发缓存数据纯内存快速构建)
    let initial_snippet_master = snippet_tree_model::build_raw_snippet_tree(&all_snippet_groups, &all_snippets);
    let mut initial_snippet_expanded = HashSet::new();
    all_snippet_groups
        .iter()
        .for_each(|g| {
            if g.is_expanded {
                initial_snippet_expanded.insert(g.id.clone());
            }
        });
    let master_snippet_tree = std::sync::Arc::new(std::sync::RwLock::new(initial_snippet_master));
    let expanded_snippet_groups = Rc::new(RefCell::new(initial_snippet_expanded));
    let snippet_search_query = Rc::new(RefCell::new(String::new()));
    let snippet_param_memory = std::sync::Arc::new(std::sync::RwLock::new(snippet_service::load_snippet_param_memory()));
    let snippet_usage_tracker = std::sync::Arc::new(std::sync::RwLock::new(snippet_service::load_snippet_usage_tracker()));

    let tunnel_search_query = Rc::new(RefCell::new(String::new()));
    let tunnel_filter_category = Rc::new(RefCell::new("all".to_string()));

    let wallpapers = Rc::new(RefCell::new(initial_config.wallpaper_list.clone()));
    let active_wallpaper_idx = Rc::new(RefCell::new(initial_config.wallpaper_active_index));
    let wallpaper_timer = Rc::new(RefCell::new(None));
    let wallpaper_preload_timer = Rc::new(RefCell::new(None));
    let wallpaper_cache = Rc::new(RefCell::new(std::collections::HashMap::new()));

    let host_store = Arc::new(crate::store::HostStore::from_arcs(
        Arc::clone(&master_tree),
        Arc::clone(&master_cards),
        Arc::clone(&expanded_groups),
        Arc::clone(&selector_expanded_groups),
        Arc::clone(&search_query),
    ));
    let ui_store = Arc::new(crate::store::UiStore::from_host_store(Arc::clone(&host_store)));

    // 注册容灾备份与多端快照同步全局后台常驻守护服务 (跟随 AppReadyEvent、资产变动与 AppBeforeExitEvent)
    let backup_daemon = std::sync::Arc::new(backup_daemon::BackupDaemonService::new(
        core_state.storage().clone(),
        window.as_weak(),
        notifications.clone(),
    ));
    backup_daemon.clone().register(core_state.event_manager());

    // 注册高性能并发文件传输队列管理中心 (默认 3 并发槽位)
    let transfer_manager = Arc::new(transfer_manager::TransferQueueManager::new(
        window.as_weak(),
        notifications.clone(),
        3,
    ));

    // 构造全局应用上下文
    let ctx = AppContext {
        core_state: Rc::clone(&core_state),
        ui_store,
        host_store,
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
        local_files_limit: Rc::new(RefCell::new(200)),
        remote_files_limit: Rc::new(RefCell::new(200)),
        transfer_tasks: Rc::clone(&transfer_tasks),
        notifications,

        master_snippet_tree,
        expanded_snippet_groups,
        snippet_search_query,
        snippet_param_memory,
        snippet_usage_tracker,

        tunnel_search_query,
        tunnel_filter_category,
        tray_active: Rc::new(RefCell::new(_tray_service.is_some())),
        backup_daemon: Arc::clone(&backup_daemon),
        sftp_driver: native_sftp_driver,
        transfer_manager,
    };

    // 初始遵循 Universal Lazy-Injected UI 架构规范：
    // 启动时默认处于 terminal + hosts 视口，仅挂载活跃视图；
    // 其余重数据页面 (files, history, snippets, tunnels) 均由 ViewLifecycleManager 按需瞬时灌入与切离卸载。



    // 统一挂载所有区域的回调事件处理器
    register_all_handlers(&window, &ctx);

    // 同步底层配置仓储 (ConfigRepository) 全量状态至 Slint SettingsBridge 视图模型
    let wb = window.global::<WindowBridge>();
    let sb = window.global::<SettingsBridge>();
    handlers::settings_handlers::apply_config_to_settings_bridge(&sb, &initial_config);
    let ai_b = window.global::<crate::generated::AiBridge>();
    ai_b.set_available_models(sb.get_setting_ai_current_models());
    ai_b.set_selected_model(sb.get_setting_ai_model());

    // 检查本地凭据保险库冷启动锁定状态 (若开启主密码且未解锁，呼出全屏解锁遮罩)
    handlers::settings_handlers::check_vault_lock_on_boot(&window, &ctx);

    // 初始化会话操作安全审计状态
    crate::audit_logger::set_audit_enabled(initial_config.session_audit_logging);

    // 启动空闲超时自动锁定保险库后台守护任务
    handlers::settings_handlers::start_auto_lock_monitor(
        window.as_weak(),
        core_state.storage().clone(),
        ctx.notifications.clone(),
    );

    wb.set_current_theme_id(initial_config.theme_id.as_str().into());
    wb.set_is_dark_mode(initial_config.is_dark_mode);
    wb.invoke_switch_theme(initial_config.theme_id.as_str().into());

    wb.set_terminal_font_family(initial_config.font_family.as_str().into());
    wb.set_terminal_font_size(initial_config.font_size);
    if let Some(ref mut r) = *terminal_renderer.borrow_mut() {
        let is_light = !initial_config.is_dark_mode || initial_config.theme_id.contains("light");
        if is_light {
            r.update_palette(terminal::TerminalPalette::light());
        }
        r.set_cursor_style(&initial_config.cursor_style);
        r.set_cursor_blink(initial_config.cursor_blink);
        if let Some(bytes) = terminal::renderer::find_font_by_name(&initial_config.font_family) {
            let _ = r.update_font(&bytes, initial_config.font_size);
        }
    }

    wb.set_is_debug_enabled(initial_config.debug_enabled);
    let dbg_bridge = window.global::<DebugBridge>();
    dbg_bridge.set_flag_desktop_notifications(initial_config.flag_desktop_notifications);
    dbg_bridge.set_flag_terminal_crt_shader(initial_config.flag_terminal_crt_shader);
    dbg_bridge.set_flag_cloud_sync(initial_config.flag_cloud_sync);
    dbg_bridge.set_flag_terminal_scratchpad(initial_config.flag_terminal_scratchpad);

    // 同步壁纸状态至全局 AppTheme 令牌与 WindowBridge
    let theme_global = window.global::<AppTheme>();
    theme_global.set_wallpaper_mode(initial_config.wallpaper_mode.as_str().into());
    theme_global.set_wallpaper_opacity(initial_config.wallpaper_opacity);
    theme_global.set_modal_opacity(initial_config.modal_opacity);

    wb.set_wallpaper_mode(initial_config.wallpaper_mode.as_str().into());
    wb.set_global_wallpaper_opacity(initial_config.wallpaper_opacity);
    wb.set_terminal_wallpaper_opacity(initial_config.wallpaper_opacity);
    wb.set_wallpaper_path(initial_config.wallpaper_path.as_str().into());

    // 壁纸画廊数据与初始渲染
    let mut initial_wp_path = initial_config.wallpaper_path.clone();
    if !initial_config.wallpaper_list.is_empty() {
        let mut active_idx = initial_config.wallpaper_active_index;
        // 若配置为 startup 开机轮播模式，冷启动时随机轮换一张新壁纸
        if initial_config.wallpaper_slideshow_interval == "startup" && initial_config.wallpaper_list.len() > 1 {
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as usize;
            let rand_offset = (seed % (initial_config.wallpaper_list.len() - 1)) + 1;
            active_idx = (initial_config.wallpaper_active_index + rand_offset) % initial_config.wallpaper_list.len();
            *ctx.active_wallpaper_idx.borrow_mut() = active_idx;
            sb.set_setting_wallpaper_active_index(active_idx as i32);

            let storage_wp = core_state.storage().clone();
            crate::async_util::spawn_async(async move {
                let _ = storage_wp.config().update(Box::new(move |c| {
                    c.wallpaper_active_index = active_idx;
                })).await;
            });
        }

        if active_idx < initial_config.wallpaper_list.len() {
            let wp_entry = &initial_config.wallpaper_list[active_idx];
            let p = std::path::Path::new(wp_entry);
            let actual_path = if p.is_dir() {
                handlers::theme_handlers::resolve_all_wallpaper_images(&[wp_entry.clone()])
                    .into_iter()
                    .next()
                    .unwrap_or_else(|| wp_entry.clone())
            } else {
                wp_entry.clone()
            };
            initial_wp_path = actual_path;
        }
    }

    wb.invoke_set_wallpaper(
        initial_config.wallpaper_mode.as_str().into(),
        initial_wp_path.as_str().into(),
        initial_config.wallpaper_opacity,
    );

    // 冷启动恢复壁纸轮播后台定时器（静默生效：不弹 Toast，不重复落盘）
    handlers::theme_handlers::apply_wallpaper_slideshow(
        &window,
        &ctx,
        initial_config.wallpaper_slideshow_interval.as_str(),
        initial_config.wallpaper_transition_effect.as_str(),
        true,
    );



    // 持久化多窗格与分割条数据模型引用（保持 ModelRc 实例单一持久，避免 Slint 重建 UI 组件导致拖拽焦点丢失）
    let panes_model = Rc::new(slint::VecModel::<TerminalPaneData>::default());
    let splitters_model = Rc::new(slint::VecModel::<TerminalSplitterData>::default());
    window.global::<TerminalBridge>().set_panes(slint::ModelRc::new(Rc::clone(&panes_model)));
    window.global::<TerminalBridge>().set_splitters(slint::ModelRc::new(Rc::clone(&splitters_model)));

    let panes_model_timer = Rc::clone(&panes_model);
    let splitters_model_timer = Rc::clone(&splitters_model);

    // 启动 120Hz (8ms) 超流畅终端位图渲染与 PTY 异步流输出泵送定时器
    let render_timer = slint::Timer::default();
    let window_weak = window.as_weak();
    let ctx_timer = ctx.clone();
    let active_terminals_timer = Rc::clone(&active_terminals);
    let terminal_renderer_timer = Rc::clone(&terminal_renderer);
    let pane_groups_timer = Rc::clone(&pane_groups);
    let global_split_tree_timer = Rc::clone(&global_split_tree);
    let active_pane_id_timer = Rc::clone(&active_pane_id);
    let zoomed_pane_id_timer = Rc::clone(&zoomed_pane_id);
    let mut last_rendered_session = String::new();
    let mut primary_ping_pong = crate::terminal::PingPongPixelBuffer::new(100, 60);
    let mut pane_ping_pongs: std::collections::HashMap<String, crate::terminal::PingPongPixelBuffer> = std::collections::HashMap::new();
    let mut pane_rendered_images: std::collections::HashMap<String, slint::Image> = std::collections::HashMap::new();
    let mut pane_tab_models: std::collections::HashMap<String, (Vec<TabData>, slint::ModelRc<TabData>)> = std::collections::HashMap::new();
    let mut last_hist_size = -1i32;
    let mut last_scroll_off = -1i32;

    render_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(16),
        move || {
            if let Some(w) = window_weak.upgrade() {
                // 0. 消费后台异步连接阶段输出与进度日志，写入对应终端视口字符流
                if let Ok(mut logs) = crate::handlers::host_handlers::PENDING_TERMINAL_LOGS.try_lock() {
                    if !logs.is_empty() {
                        let items: Vec<_> = logs.drain(..).collect();
                        drop(logs);
                        let mut terminals = active_terminals_timer.borrow_mut();
                        for (sess_id, msg) in items {
                            if let Some(inst) = terminals.get_mut(&sess_id) {
                                inst.parser.process(msg.as_bytes());
                                inst.parser.mark_dirty();
                            }
                        }
                    }
                }

                // 1. 消费后台异步完成的 SSH 握手连接任务，挂载终端实例并更新指示灯状态为 online/error
                if let Ok(mut pending) = crate::handlers::host_handlers::PENDING_SSH_CONNECTIONS.try_lock() {
                    if !pending.is_empty() {
                        let items: Vec<_> = pending.drain(..).collect();
                        drop(pending);
                        for (sess_id, res, launch_cfg_opt) in items {
                            match res {
                                Ok(mut ready_instance) => {
                                    let mut terminals = active_terminals_timer.borrow_mut();
                                    if let Some(existing) = terminals.get_mut(&sess_id) {
                                        existing.pty = ready_instance.pty;
                                        existing.key_guard = ready_instance.key_guard;
                                        existing.target = ready_instance.target;
                                        existing.state = crate::terminal::instance::SessionState::Running;
                                        let _ = existing.pty.resize(existing.size);
                                        let ok_msg = "\x1b[32m[smalux] 连接成功!\x1b[0m\r\n\r\n".as_bytes();
                                        existing.parser.process(ok_msg);
                                        existing.parser.mark_dirty();
                                    } else {
                                        let ok_msg = "\x1b[32m[smalux] 连接成功!\x1b[0m\r\n\r\n".as_bytes();
                                        ready_instance.parser.process(ok_msg);
                                        ready_instance.parser.mark_dirty();
                                        terminals.insert(sess_id.clone(), ready_instance);
                                    }
                                    let mut groups = pane_groups_timer.borrow_mut();
                                    for g in groups.iter_mut() {
                                        if let Some(t) = g.tabs.iter_mut().find(|t| t.session_id == sess_id) {
                                            t.host_status = "online".to_string();
                                        }
                                    }
                                    let active_pid = active_pane_id_timer.borrow().clone();
                                    let is_split = global_split_tree_timer.borrow().is_some();
                                    session::sync_active_session_ui(&w, &groups, &active_pid, is_split);
                                    session::sync_active_session_to_core(&groups, &active_pid, &ctx_timer.core_state);
                                    tracing::info!(target: "smagical_ui::session", "后台异步 SSH 连接就绪挂载: {}", sess_id);
                                }
                                Err(err) => {
                                    let is_auth_error = err.contains("认证失败")
                                        || err.contains("AuthFailed")
                                        || err.contains("Permission denied")
                                        || err.contains("拒绝用户");
                                    let mut terminals = active_terminals_timer.borrow_mut();
                                    let mut is_still_reconnecting = false;
                                    if let Some(existing) = terminals.get_mut(&sess_id) {
                                        if let Some(cfg) = launch_cfg_opt {
                                            if let crate::terminal::instance::TerminalTarget::Ssh { ref mut config, .. } = existing.target {
                                                *config = Some(cfg);
                                            }
                                        }
                                        let fail_msg = format!("\x1b[31m[smalux] 连接失败: {}\x1b[0m\r\n", err);
                                        existing.parser.process(fail_msg.as_bytes());
                                        existing.parser.mark_dirty();

                                        let retry_plan = if is_auth_error {
                                            // 认证失败严禁自动重连，防止服务器封禁 IP 并停止无意义重试
                                            None
                                        } else {
                                            match &existing.state {
                                                crate::terminal::instance::SessionState::Reconnecting { attempt, max_attempts, .. } => {
                                                    if *attempt < *max_attempts {
                                                        let next_att = attempt + 1;
                                                        let delay = (1u64 << (next_att - 1)).min(15);
                                                        Some((next_att, *max_attempts, delay))
                                                    } else {
                                                        None
                                                    }
                                                }
                                                _ => None,
                                            }
                                        };

                                        if let Some((next_att, max_att, delay)) = retry_plan {
                                            is_still_reconnecting = true;
                                            existing.enter_reconnecting(next_att, max_att, delay);
                                        } else {
                                            let exit_reason = if is_auth_error {
                                                crate::terminal::instance::SessionExitReason::AuthFailed
                                            } else {
                                                crate::terminal::instance::SessionExitReason::Disconnected
                                            };
                                            existing.state = crate::terminal::instance::SessionState::Exited {
                                                reason: exit_reason,
                                                exited_at: std::time::Instant::now(),
                                            };
                                            if matches!(existing.target, crate::terminal::instance::TerminalTarget::Ssh { .. }) {
                                                if is_auth_error {
                                                    existing.parser.process("\r\n\x1b[90m[smalux] 认证失败已停止自动重新连接，请更新凭据配置后重试 (Ctrl+W 关闭标签页)\x1b[0m\r\n".as_bytes());
                                                } else {
                                                    existing.parser.process("\r\n\x1b[90m[smalux] 按任意键手动重新连接，或按 Ctrl+W 关闭标签页\x1b[0m\r\n".as_bytes());
                                                }
                                                existing.parser.mark_dirty();
                                            }
                                        }
                                    }
                                    {
                                        let mut groups = pane_groups_timer.borrow_mut();
                                        for g in groups.iter_mut() {
                                            if let Some(t) = g.tabs.iter_mut().find(|t| t.session_id == sess_id) {
                                                t.host_status = if is_still_reconnecting { "warning".to_string() } else { "error".to_string() };
                                            }
                                        }
                                    }
                                    let groups = pane_groups_timer.borrow();
                                    let active_pid = active_pane_id_timer.borrow().clone();
                                    let is_split = global_split_tree_timer.borrow().is_some();
                                    session::sync_active_session_ui(&w, &groups, &active_pid, is_split);
                                    if !is_still_reconnecting {
                                        ctx_timer.notify_error("SSH 连接失败", err.clone());
                                    }
                                    let storage_for_hist = ctx_timer.core_state.storage().clone();
                                    let hist_id = format!("hist-{}", sess_id);
                                    let err_clone = err.clone();
                                    crate::async_util::spawn_async(async move {
                                        if let Ok(Some(mut hist)) = storage_for_hist.history().get_by_id(&hist_id).await {
                                            let now_s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();
                                            let reason = if is_auth_error { "auth_failed" } else { "error" };
                                            hist.mark_failed(now_s, reason, Some(err_clone));
                                            let _ = storage_for_hist.history().save(&hist).await;
                                            crate::handlers::history_handlers::invalidate_history_cache();
                                        }
                                    });
                                }
                            }
                        }
                    }
                }

                if pane_groups_timer.borrow().is_empty() {
                    return;
                }

                let mut terminals = active_terminals_timer.borrow_mut();
                let auto_reconnect_enabled = w.global::<crate::generated::SettingsBridge>().get_setting_auto_reconnect();
                let is_terminal_view = w.global::<WindowBridge>().get_main_view() == "terminal";
                let is_split = global_split_tree_timer.borrow().is_some();

                // 2. 统计当前屏幕上处于激活/可见状态的终端会话 ID 集合 (局域借用，计算完毕即时释放)
                let visible_sess_ids = {
                    let groups = pane_groups_timer.borrow();
                    let mut ids = std::collections::HashSet::new();
                    if is_terminal_view {
                        if !is_split {
                            if let Some(act) = groups.first().and_then(|g| g.get_active_session()) {
                                ids.insert(act.session_id.clone());
                            }
                        } else if let Some(ref tree) = *global_split_tree_timer.borrow() {
                            for pane_id in tree.all_pane_ids() {
                                if let Some(g) = groups.iter().find(|g| g.pane_id == pane_id) {
                                    if let Some(act) = g.get_active_session() {
                                        ids.insert(act.session_id.clone());
                                    }
                                }
                            }
                        }
                    }
                    ids
                };

                // 3. 后台终端全量保活与管道排空：对所有未被屏幕渲染的实例消费输出并实时探测断线
                for (sess_id, instance) in terminals.iter_mut() {
                    if !visible_sess_ids.contains(sess_id) {
                        let _ = instance.poll_output();
                    }
                }

                // 4. 自动重连调度
                let mut auto_reconnect_tasks = Vec::new();

                for (sess_id, instance) in terminals.iter_mut() {
                    // 若开启了自动重连且会话由于网络断开退出 (且未发生认证失败)，自动进入 Reconnecting 状态 (第 1 次，1 秒后重试)
                    if auto_reconnect_enabled && !instance.auth_failed && matches!(instance.target, crate::terminal::instance::TerminalTarget::Ssh { .. }) {
                        if let crate::terminal::instance::SessionState::Exited { reason: crate::terminal::instance::SessionExitReason::Disconnected, exited_at } = &instance.state {
                            if exited_at.elapsed() >= std::time::Duration::from_millis(500) {
                                instance.enter_reconnecting(1, 3, 1);
                            }
                        }
                    }

                    // 若会话处于 Reconnecting 状态且重试倒计时已到，发起异步重新连接管线
                    if let crate::terminal::instance::SessionState::Reconnecting { attempt, max_attempts, next_attempt_at } = &mut instance.state {
                        if std::time::Instant::now() >= *next_attempt_at {
                            *next_attempt_at = std::time::Instant::now() + std::time::Duration::from_secs(60);
                            let msg = format!("\x1b[33m[smalux] 正在发起自动重新连接 (第 {}/{} 次)...\x1b[0m\r\n", attempt, max_attempts);
                            instance.parser.process(msg.as_bytes());
                            instance.parser.mark_dirty();
                            instance.pty = crate::terminal::TerminalBackend::Connecting(instance.size);

                            let s_info_opt = pane_groups_timer.borrow().iter().find_map(|g| g.tabs.iter().find(|t| t.session_id == *sess_id).cloned());
                            if let Some(s_info) = s_info_opt {
                                auto_reconnect_tasks.push((sess_id.clone(), s_info, instance.size.cols, instance.size.rows));
                            }
                        }
                    }
                }

                // 5. 状态同步：统一在可变借用中完成比对与就地更新，杜绝跨作用域重叠借用导致 RefCell already borrowed 崩溃
                {
                    let mut any_status_changed = false;
                    {
                        let mut groups_mut = pane_groups_timer.borrow_mut();
                        for g in groups_mut.iter_mut() {
                            for t in g.tabs.iter_mut() {
                                if let Some(inst) = terminals.get(&t.session_id) {
                                    let cur_st = inst.current_status();
                                    if t.host_status != cur_st {
                                        t.host_status = cur_st.to_string();
                                        any_status_changed = true;
                                    }
                                }
                            }
                        }
                    }
                    if any_status_changed {
                        let groups = pane_groups_timer.borrow();
                        let active_pid = active_pane_id_timer.borrow().clone();
                        session::sync_active_session_ui(&w, &groups, &active_pid, is_split);
                    }
                }

                for (s_id, s_info, cols, rows) in auto_reconnect_tasks {
                    let (addr, port) = if let Some((a, p)) = s_info.host_address.split_once(':') {
                        (a.to_string(), p.parse::<u16>().unwrap_or(22))
                    } else {
                        (s_info.host_address.clone(), 22)
                    };
                    let storage = ctx_timer.core_state.storage().clone();
                    let ssh_svc = ctx_timer.core_state.ssh().clone();
                    crate::handlers::host_handlers::spawn_ssh_connection_pipeline(
                        s_id,
                        s_info.display_title,
                        s_info.host_id,
                        s_info.host_name,
                        addr,
                        port,
                        None,
                        storage,
                        ssh_svc,
                        cols,
                        rows,
                    );
                }

                // 检查当前主视图是否为终端工作区
                if !is_terminal_view {
                    return;
                }

                let groups = pane_groups_timer.borrow();
                let is_split = global_split_tree_timer.borrow().is_some();
                let mut renderer_opt = terminal_renderer_timer.borrow_mut();
                let mut status_change: Option<(String, String)> = None;

                if !is_split {
                    // 回收切回单屏模式后遗留的多分屏像素缓冲区与 GPU 纹理显存
                    if !pane_ping_pongs.is_empty() {
                        pane_ping_pongs.clear();
                    }
                    if !pane_rendered_images.is_empty() {
                        pane_rendered_images.clear();
                    }
                    if !pane_tab_models.is_empty() {
                        pane_tab_models.clear();
                    }

                    // 1. 单屏模式：泵送并渲染主视口
                    let main_group = &groups[0];
                    if let Some(active_sess) = main_group.get_active_session() {
                        let active_id = active_sess.session_id.clone();
                        let tb = w.global::<TerminalBridge>();
                        let ui_cols = tb.get_terminal_cols() as u16;
                        let ui_rows = tb.get_terminal_rows() as u16;

                        if let Some(instance) = terminals.get_mut(&active_id) {
                            // 仅在获取到合理的有效视口行列尺寸时才触发底层 PTY resize，杜绝 0 尺寸 ConPTY 震荡
                            if ui_cols >= 10 && ui_rows >= 5 && (instance.size.cols != ui_cols || instance.size.rows != ui_rows) {
                                let _ = instance.resize(ui_cols, ui_rows);
                            }

                            let has_new_output = instance.poll_output();
                            let is_dirty = instance.parser.take_dirty();

                            let cur_st = instance.current_status();
                            if active_sess.host_status != cur_st {
                                status_change = Some((active_id.clone(), cur_st.to_string()));
                            }

                            if let Some(renderer) = renderer_opt.as_mut() {
                                let (cw, ch) = renderer.cell_size();
                                let render_cols = if ui_cols >= 10 { ui_cols } else { instance.size.cols };
                                let render_rows = if ui_rows >= 5 { ui_rows } else { instance.size.rows };
                                let img_w = (render_cols as u32 * cw + renderer.padding_x * 2).max(100);
                                let img_h = (render_rows as u32 * ch + renderer.padding_y * 2).max(60);

                                primary_ping_pong.resize(img_w, img_h);

                                if has_new_output || is_dirty || last_rendered_session != active_id {
                                    renderer.render_to_ping_pong(instance.parser.term(), instance.parser.selection(), &mut primary_ping_pong, &active_id);
                                    let image = primary_ping_pong.commit_to_image();
                                    tb.set_terminal_screen_image(image);
                                    last_rendered_session = active_id;
                                }
                            }

                            let (hist_size, scroll_off) = instance.scroll_info();
                            let h_i = hist_size as i32;
                            let s_i = scroll_off as i32;
                            if last_hist_size != h_i {
                                tb.set_history_size(h_i);
                                last_hist_size = h_i;
                            }
                            if last_scroll_off != s_i {
                                tb.set_scroll_offset(s_i);
                                last_scroll_off = s_i;
                            }
                        }
                    }
                } else {
                    // 2. 任意层级嵌套二叉多分屏模式：数据驱动推导几何并渲染全量活跃叶子窗格
                    let trees_guard = global_split_tree_timer.borrow();
                    if let Some(tree) = trees_guard.as_ref() {
                        let tb = w.global::<TerminalBridge>();
                        let vp_w = tb.get_canvas_width().max(200.0);
                        let vp_h = tb.get_canvas_height().max(100.0);
                        let current_zoom = zoomed_pane_id_timer.borrow().clone();
                        let (panes_layout, splitters_layout) = tree.compute_pixel_layout(vp_w, vp_h, 2.0, current_zoom.as_deref());

                        let active_pid = active_pane_id_timer.borrow().clone();
                        let total_panes_len = panes_layout.len();
                        let mut panes_data = Vec::with_capacity(total_panes_len);

                        for (idx, pl) in panes_layout.iter().enumerate() {
                            let mut pane_image = slint::Image::default();
                            let group_opt = groups.iter().find(|g| g.pane_id == pl.pane_id);
                            let active_sess_opt = group_opt.and_then(|g| g.get_active_session());

                            let mut hist_size = 0i32;
                            let mut scroll_off = 0i32;
                            let mut term_rows = 24i32;

                            if let Some(active_sess) = active_sess_opt
                                && let Some(instance) = terminals.get_mut(&active_sess.session_id)
                            {
                                if let Some(renderer) = renderer_opt.as_mut() {
                                    let (cw, ch) = renderer.cell_size();
                                    let title_bar_h = 36.0f32;
                                    let content_w = (pl.width - (renderer.padding_x * 2) as f32).max(40.0);
                                    let content_h = (pl.height - title_bar_h - (renderer.padding_y * 2) as f32).max(20.0);

                                    let target_cols = ((content_w / cw as f32) as u16).max(10);
                                    let target_rows = ((content_h / ch as f32) as u16).max(3);
                                    term_rows = target_rows as i32;

                                    if instance.size.cols != target_cols || instance.size.rows != target_rows {
                                        let _ = instance.resize(target_cols, target_rows);
                                    }

                                    let has_new_output = instance.poll_output();
                                    let is_dirty = instance.parser.take_dirty();

                                    let cur_st = instance.current_status();
                                    if active_sess.host_status != cur_st {
                                        status_change = Some((active_sess.session_id.clone(), cur_st.to_string()));
                                    }

                                    let img_w = (target_cols as u32 * cw + renderer.padding_x * 2).max(50);
                                    let img_h = (target_rows as u32 * ch + renderer.padding_y * 2).max(30);

                                    let ping_pong = pane_ping_pongs
                                        .entry(pl.pane_id.clone())
                                        .or_insert_with(|| crate::terminal::PingPongPixelBuffer::new(img_w, img_h));
                                    ping_pong.resize(img_w, img_h);

                                    if has_new_output || is_dirty || !pane_rendered_images.contains_key(&pl.pane_id) {
                                        renderer.render_to_ping_pong(instance.parser.term(), instance.parser.selection(), ping_pong, &active_sess.session_id);
                                        let img = ping_pong.commit_to_image();
                                        pane_rendered_images.insert(pl.pane_id.clone(), img.clone());
                                        pane_image = img;
                                    } else if let Some(cached_img) = pane_rendered_images.get(&pl.pane_id) {
                                        pane_image = cached_img.clone();
                                    }
                                }

                                let (hs, so) = instance.scroll_info();
                                hist_size = hs as i32;
                                scroll_off = so as i32;
                                if pl.pane_id == active_pid {
                                    if last_hist_size != hist_size {
                                        tb.set_history_size(hist_size);
                                        last_hist_size = hist_size;
                                    }
                                    if last_scroll_off != scroll_off {
                                        tb.set_scroll_offset(scroll_off);
                                        last_scroll_off = scroll_off;
                                    }
                                }
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
                                    let rc = to_model_rc(pane_tabs.clone());
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
                                history_size: hist_size,
                                scroll_offset: scroll_off,
                                terminal_rows: term_rows,
                            });
                        }


                        update_model_in_place(&panes_model_timer, panes_data);

                        // 帧级 GC 回收：自动清理已被关闭窗格的未压缩像素缓冲区 (8MB+/窗格) 与 GPU 显存纹理
                        let active_pids: std::collections::HashSet<&str> = panes_layout.iter().map(|pl| pl.pane_id.as_str()).collect();
                        pane_ping_pongs.retain(|k, _| active_pids.contains(k.as_str()));
                        pane_rendered_images.retain(|k, _| active_pids.contains(k.as_str()));
                        pane_tab_models.retain(|k, _| active_pids.contains(k.as_str()));

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

                drop(terminals);
                drop(renderer_opt);
                drop(groups);

                if let Some((target_sess_id, new_status)) = status_change {
                    {
                        let mut groups_mut = pane_groups_timer.borrow_mut();
                        for grp in groups_mut.iter_mut() {
                            for t in grp.tabs.iter_mut() {
                                if t.session_id == target_sess_id {
                                    t.host_status = new_status.clone();
                                }
                            }
                        }
                    }
                    let groups = pane_groups_timer.borrow();
                    let act_pid = active_pane_id_timer.borrow().clone();
                    let is_split_now = global_split_tree_timer.borrow().is_some();
                    sync_active_session_ui(&w, &groups, &act_pid, is_split_now);
                }
            }
        },
    );




    // 触发全局应用界面首帧就绪事件
    core_state.events().dispatch(&smagical_core::AppReadyEvent);

    window.show()?;
    slint::run_event_loop_until_quit()?;
    Ok(())

}

pub(crate) use store::diff::update_model_in_place;


