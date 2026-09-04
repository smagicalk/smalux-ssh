//! 窗口级通用交互（主题切换、深浅色模式、系统窗口最小化/最大化/关闭）回调绑定。
//!
//! 提供跨平台通用桌面窗口状态管理与主题系统的事件响应。

use std::rc::Rc;
use slint::ComponentHandle;
use slint::winit_030::WinitWindowAccessor;
use smagical_core::event::{
    AppBeforeExitEvent, AppExitEvent, ConfigChangedEvent, ThemeChangedEvent,
    ThemeModeToggledEvent, WindowStateChangedEvent,
};
use theme::apply_theme_by_id;

use crate::generated::AppWindow;
use crate::handlers::AppContext;
use crate::{theme, AppTheme};

#[cfg(target_os = "windows")]
fn is_autostart_enabled() -> bool {
    std::process::Command::new("reg")
        .args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run", "/v", "smalux-ssh"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
fn is_autostart_enabled() -> bool {
    false
}

#[cfg(target_os = "windows")]
fn set_autostart_enabled(enabled: bool) -> std::io::Result<()> {
    if enabled {
        let exe_path = std::env::current_exe()?;
        let path_str = exe_path.to_string_lossy();
        let val = format!("\"{}\"", path_str);
        std::process::Command::new("reg")
            .args(["add", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run", "/v", "smalux-ssh", "/t", "REG_SZ", "/d", &val, "/f"])
            .output()?;
    } else {
        let _ = std::process::Command::new("reg")
            .args(["delete", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run", "/v", "smalux-ssh", "/f"])
            .output();
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn set_autostart_enabled(_enabled: bool) -> std::io::Result<()> {
    Ok(())
}

/// 注册窗口级通用交互回调。
///
/// 绑定主题配置切换、深色/浅色模式切换与原生无边框窗口控制三键事件。
///
/// # 参数
/// - `window`: Slint 主窗口句柄引用
/// - `ctx`: 全局应用共享上下文对象引用
pub(crate) fn register_window_handlers(window: &AppWindow, ctx: &AppContext) {
    // -------------------------------------------------------------------------
    // 1. 切换主题配色方案回调
    // -------------------------------------------------------------------------
    // 当在左侧设置抽屉或命令面板中选择某个主题 ID 时触发，实时更新 Slint UI 设计令牌与终端颜色映射。
    let window_weak = window.as_weak();
    let themes_clone = Rc::clone(&ctx.themes);
    let core_state_theme = ctx.core_state.clone();
    window.on_switch_theme(move |theme_id| {

        if let Some(w) = window_weak.upgrade() {
            let id_str = theme_id.as_str();
            let normalized_id = match id_str {
                "builtin.ui.onedark" | "onedark" => "builtin.ui.one-dark",
                "builtin.ui.solarized" | "solarized" => "builtin.ui.solarized-dark",
                "darcula" => "builtin.ui.darcula",
                "monokai" => "builtin.ui.monokai",
                "nord" => "builtin.ui.nord",
                "github-light" => "builtin.ui.github-light",
                "github-dark" => "builtin.ui.github-dark",
                "system" => "builtin.ui.system",
                other => other,
            };

            let name = if normalized_id.contains("darcula") {
                "Darcula"
            } else if normalized_id.contains("monokai") {
                "Monokai"
            } else if normalized_id.contains("one-dark") || normalized_id.contains("onedark") {
                "One Dark"
            } else if normalized_id.contains("solarized-light") {
                "Solarized Light"
            } else if normalized_id.contains("solarized") {
                "Solarized"
            } else if normalized_id.contains("nord") {
                "Nord"
            } else if normalized_id.contains("github-light") || normalized_id.contains("light") {
                "GitHub Light"
            } else if normalized_id.contains("github-dark") {
                "GitHub Dark"
            } else {
                "System"
            };

            match apply_theme_by_id(&w, &*themes_clone.borrow(), normalized_id) {
                Ok(()) => {
                    w.set_current_theme_id(normalized_id.into());
                    w.set_current_theme_name(name.into());
                    let is_light = normalized_id.contains("light") || normalized_id.contains("dawn") || normalized_id.contains("latte");
                    w.set_is_dark_mode(!is_light);
                    core_state_theme.events().dispatch(&ThemeChangedEvent {
                        theme_id: normalized_id.to_string(),
                        is_dark: !is_light,
                    });
                    core_state_theme.events().dispatch(&ConfigChangedEvent {
                        key: "appearance.theme".into(),
                        old_val: "".into(),
                        new_val: normalized_id.to_string(),
                        source: "switch_theme".into(),
                    });
                    let id_for_cfg = normalized_id.to_string();
                    let _ = core_state_theme.storage().config().update(Box::new(move |c| {
                        c.theme_id = id_for_cfg;
                        c.is_dark_mode = !is_light;
                    }));
                    tracing::info!(target: "smagical_ui::theme", "切换应用配色主题: {} ({})", name, normalized_id);

                }
                Err(err) => {
                    tracing::error!(target: "smagical_ui::theme", "切换应用主题失败 [{}]: {:?}", normalized_id, err);
                }
            }
        }
    });

    // -------------------------------------------------------------------------
    // 2. 深色 / 浅色模式一键切换回调
    // -------------------------------------------------------------------------
    // 点击顶部栏或设置中的深浅色切换图标时触发，快速在默认深色 (Darcula) 与默认浅色 (GitHub Light) 之间翻转。
    let window_weak = window.as_weak();
    let themes_clone = Rc::clone(&ctx.themes);
    let core_state_color_mode = ctx.core_state.clone();
    window.on_toggle_color_mode(move || {
        if let Some(w) = window_weak.upgrade() {
            let is_dark = w.get_is_dark_mode();
            let next_dark = !is_dark;

            if next_dark {
                let _ = apply_theme_by_id(&w, &*themes_clone.borrow(), "builtin.ui.darcula");
                w.set_current_theme_id("builtin.ui.darcula".into());
                w.set_current_theme_name("Darcula".into());
                w.set_is_dark_mode(true);
            } else {
                let _ = apply_theme_by_id(&w, &*themes_clone.borrow(), "builtin.ui.github-light");
                w.set_current_theme_id("builtin.ui.github-light".into());
                w.set_current_theme_name("GitHub Light".into());
                w.set_is_dark_mode(false);
            }

            core_state_color_mode.events().dispatch(&ThemeModeToggledEvent {
                is_dark: next_dark,
            });
            core_state_color_mode.events().dispatch(&ConfigChangedEvent {
                key: "appearance.color_mode".into(),
                old_val: if is_dark { "dark" } else { "light" }.into(),
                new_val: if next_dark { "dark" } else { "light" }.into(),
                source: "toggle_color_mode".into(),
            });

            tracing::info!(target: "smagical_ui::theme", "{}", if next_dark { "切换至深色模式 (Darcula)" } else { "切换至浅色模式 (GitHub Light)" });
        }
    });

    // -------------------------------------------------------------------------
    // 2.1 开发者调试模式全局开关切换回调
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let core_state_debug = ctx.core_state.clone();
    window.on_toggle_debug_enabled(move |enabled| {
        smagical_debug::set_debug_enabled(enabled);
        core_state_debug.activity_bar().set_visible("debug", enabled);
        if let Some(w) = window_weak.upgrade() {
            w.set_is_debug_enabled(enabled);
            crate::activity_bar_service::sync_activity_bar_ui(&w, &core_state_debug);
            if !enabled {
                w.set_is_debug_modal_open(false);
                if w.get_active_left_tab() == "debug" {
                    w.set_active_left_tab("hosts".into());
                }
            } else {
                crate::debug_ui::sync_ui_debug_logs(&w);
            }
            core_state_debug.events().dispatch(&ConfigChangedEvent {
                key: "developer.debug_enabled".into(),
                old_val: if enabled { "false" } else { "true" }.into(),
                new_val: if enabled { "true" } else { "false" }.into(),
                source: "toggle_debug_enabled".into(),
            });
            let _ = core_state_debug.storage().config().update(Box::new(move |c| {
                c.debug_enabled = enabled;
            }));
            tracing::info!(target: "smagical_ui::settings", "开发者调试控制台已{}", if enabled { "开启" } else { "关闭" });
        }
    });

    // -------------------------------------------------------------------------
    // 2.2 全局统一路由跳转导航回调 (Navigation Router)
    // -------------------------------------------------------------------------
    let core_state_nav = ctx.core_state.clone();
    window.on_navigate_to(move |target_tab, section| {
        let t_str = target_tab.as_str();
        let s_str = section.as_str();
        let mut req = smagical_core::NavigationRequest::target(t_str);
        if !s_str.is_empty() {
            req = req.with_section(s_str);
        }
        core_state_nav.navigate_to(req);
        tracing::info!(target: "smagical_ui::navigation", "路由中枢成功处理跳转请求: [{}] (section: {:?})", t_str, if s_str.is_empty() { None } else { Some(s_str) });
    });


    // 同步初始化系统开机自启状态
    window.set_setting_start_on_boot(is_autostart_enabled());

    // -------------------------------------------------------------------------
    // 3. 窗口控制: 关闭应用 (带活跃会话前置拦截、托盘保护与退出归档)
    // -------------------------------------------------------------------------
    // 点击右上角红色关闭按钮时，根据配置执行活跃会话拦截、托盘最小化或安全退出。
    let window_weak = window.as_weak();
    let core_state_close = ctx.core_state.clone();
    let pane_groups_close = Rc::clone(&ctx.pane_groups);
    let persistence_guard_close = std::sync::Arc::clone(&ctx.persistence_guard);
    let notif_close = ctx.notifications.clone();
    window.on_close_window(move || {
        if let Some(w) = window_weak.upgrade() {
            let mut remote_count = 0;
            let mut local_count = 0;
            for g in pane_groups_close.borrow().iter() {
                for t in &g.tabs {
                    if t.host_id.starts_with("local-") || t.host_address.starts_with("Local") {
                        local_count += 1;
                    } else {
                        remote_count += 1;
                    }
                }
            }
            let active_count = remote_count + local_count;

            // A. 如果开启了活跃会话防呆确认，且当前有活跃会话（优先拦截弹窗）
            if active_count > 0 && w.get_setting_confirm_close_active() {
                w.set_is_exit_confirm_open(true);
                tracing::info!(
                    target: "smagical_ui::window",
                    "检测到 {} 个远程 SSH 会话和 {} 个本地终端正在运行，拦截关闭并弹出二次确认",
                    remote_count, local_count
                );
                return;
            }

            // B. 如果设置关闭时最小化到系统托盘 (tray)
            if w.get_setting_close_action() == "tray" {
                w.window().set_minimized(true);
                notif_close.info("已最小化到后台", "网络隧道与 SSH 会话在后台持续保持连接中");
                tracing::info!(target: "smagical_ui::window", "窗口关闭动作已转为托盘后台运行");
                return;
            }

            // C. 正常直接退出
            let before_exit_event = AppBeforeExitEvent::new(active_count);
            core_state_close.events().dispatch(&before_exit_event);
            if before_exit_event.is_aborted() {
                tracing::warn!(
                    target: "smagical_ui::window",
                    "应用退出流程被安全守护拦截: {:?}",
                    before_exit_event.abort_reason()
                );
                return;
            }
            persistence_guard_close.flush_and_wait(std::time::Duration::from_millis(1000));
            core_state_close.events().dispatch(&AppExitEvent { exit_code: 0 });
            std::process::exit(0);
        }
    });

    // -------------------------------------------------------------------------
    // 3.0 系统原生关闭事件生命周期拦截 (Alt+F4 / 任务栏关闭等原生事件)
    // -------------------------------------------------------------------------
    let window_weak_req = window.as_weak();
    let pane_groups_req = Rc::clone(&ctx.pane_groups);
    window.window().on_close_requested(move || -> slint::CloseRequestResponse {
        if let Some(w) = window_weak_req.upgrade() {
            let active_count: usize = pane_groups_req.borrow().iter().map(|g| g.tabs.len()).sum();
            if active_count > 0 && w.get_setting_confirm_close_active() {
                w.set_is_exit_confirm_open(true);
                return slint::CloseRequestResponse::KeepWindowShown;
            }
            if w.get_setting_close_action() == "tray" {
                w.window().set_minimized(true);
                return slint::CloseRequestResponse::KeepWindowShown;
            }
        }
        slint::CloseRequestResponse::HideWindow
    });

    // -------------------------------------------------------------------------
    // 3.1 强制退出应用回调 (用户在二次防呆弹窗中点击“强行退出”)
    // -------------------------------------------------------------------------
    let core_state_force = ctx.core_state.clone();
    let persistence_guard_force = std::sync::Arc::clone(&ctx.persistence_guard);
    window.on_force_close_window(move || {
        persistence_guard_force.flush_and_wait(std::time::Duration::from_millis(1000));
        core_state_force.events().dispatch(&AppExitEvent { exit_code: 0 });
        std::process::exit(0);
    });

    // -------------------------------------------------------------------------
    // 3.2 窗口控制: 窗口始终置顶 (Always on Top)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let notif_top = ctx.notifications.clone();
    window.on_toggle_always_on_top(move |always_on_top| {
        if let Some(w) = window_weak.upgrade() {
            w.set_setting_always_on_top(always_on_top);
            w.window().with_winit_window(|winit_window| {
                let level = if always_on_top {
                    slint::winit_030::winit::window::WindowLevel::AlwaysOnTop
                } else {
                    slint::winit_030::winit::window::WindowLevel::Normal
                };
                winit_window.set_window_level(level);
            });
            if always_on_top {
                notif_top.info("窗口置顶已开启", "客户端窗口将始终显示在其他应用之上");
            } else {
                notif_top.info("窗口置顶已关闭", "客户端窗口已恢复普通层级");
            }
            tracing::info!(target: "smagical_ui::window", "窗口置顶状态设置为: {}", always_on_top);
        }
    });

    // -------------------------------------------------------------------------
    // 3.3 系统开机自启设置 (Launch on Boot)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let notif_boot = ctx.notifications.clone();
    let core_state_boot = ctx.core_state.clone();
    window.on_toggle_start_on_boot(move |enabled| {
        if let Some(w) = window_weak.upgrade() {
            w.set_setting_start_on_boot(enabled);
            let _ = core_state_boot.storage().config().update(Box::new(move |c| {
                c.start_on_boot = enabled;
            }));
            match set_autostart_enabled(enabled) {
                Ok(()) => {
                    if enabled {
                        notif_boot.success("开机自启已开启", "客户端已添加至 Windows 登录自启动列表");
                    } else {
                        notif_boot.info("开机自启已关闭", "已从 Windows 登录自启动列表中移除");
                    }
                    tracing::info!(target: "smagical_ui::settings", "开机自启设置为: {}", enabled);
                }
                Err(e) => {
                    notif_boot.error("设置开机自启失败", &format!("{}", e));
                    tracing::error!(target: "smagical_ui::settings", "设置开机自启失败: {}", e);
                }
            }
        }
    });

    // -------------------------------------------------------------------------
    // 3.4 退出时含活跃会话防呆确认设置
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    let core_state_confirm = ctx.core_state.clone();
    window.on_toggle_confirm_close_active(move |enabled| {
        if let Some(w) = window_weak.upgrade() {
            w.set_setting_confirm_close_active(enabled);
            let _ = core_state_confirm.storage().config().update(Box::new(move |c| {
                c.confirm_close_active = enabled;
            }));
            tracing::info!(target: "smagical_ui::settings", "退出时活跃会话防呆确认设置为: {}", enabled);
        }
    });

    // -------------------------------------------------------------------------
    // 3.5 切换图形渲染引擎及重启确认拦截
    // -------------------------------------------------------------------------
    let pending_pipeline_restart = Rc::new(std::cell::RefCell::new(Option::<String>::None));

    let window_weak_pipe = window.as_weak();
    let pending_pipe_for_switch = Rc::clone(&pending_pipeline_restart);
    window.on_switch_rendering_pipeline(move |pipe_id| {
        if let Some(w) = window_weak_pipe.upgrade() {
            let p_str = pipe_id.to_string();
            let cur = w.get_active_rendering_pipeline().to_string();
            if cur == p_str {
                return;
            }

            let pipe_name = match p_str.as_str() {
                "winit-skia" => "Skia Auto (推荐)",
                "winit-skia-opengl" => "Skia OpenGL",
                "winit-skia-vulkan" => "Skia Vulkan",
                "winit-skia-software" => "CPU 软件安全渲染",
                _ => p_str.as_str(),
            };

            *pending_pipe_for_switch.borrow_mut() = Some(p_str.clone());
            w.set_restart_confirm_message(
                format!("切换渲染引擎为 [{}] 需要重启客户端以完成底层 GPU 显卡管线重新绑定。是否立即重启？", pipe_name).into()
            );
            w.set_is_restart_confirm_open(true);
            tracing::info!(target: "smagical_ui::settings", "请求切换渲染引擎为: {}，已唤起重启确认弹窗", p_str);
        }
    });

    let window_weak_restart = window.as_weak();
    let pending_pipe_for_restart = Rc::clone(&pending_pipeline_restart);
    let persistence_guard_restart = std::sync::Arc::clone(&ctx.persistence_guard);
    window.on_confirm_restart_pipeline(move || {
        if let Some(w) = window_weak_restart.upgrade() {
            if let Some(p_str) = pending_pipe_for_restart.borrow_mut().take() {
                w.set_active_rendering_pipeline(p_str.clone().into());
                unsafe {
                    std::env::set_var("SLINT_BACKEND", &p_str);
                }
                tracing::info!(target: "smagical_ui::settings", "正在执行客户端安全重启以生效全新渲染管线: [{}]...", p_str);

                persistence_guard_restart.flush_and_wait(std::time::Duration::from_millis(500));

                if let Ok(exe_path) = std::env::current_exe() {
                    let mut cmd = std::process::Command::new(exe_path);
                    cmd.env("SLINT_BACKEND", &p_str);
                    if let Err(e) = cmd.spawn() {
                        tracing::error!(target: "smagical_ui::settings", "重启客户端拉起新进程失败: {:?}", e);
                    } else {
                        std::process::exit(0);
                    }
                }
            }
        }
    });

    let window_weak_cancel = window.as_weak();
    let pending_pipe_for_cancel = Rc::clone(&pending_pipeline_restart);
    let notif_pipe_cancel = ctx.notifications.clone();
    window.on_cancel_restart_pipeline(move || {
        if let Some(w) = window_weak_cancel.upgrade() {
            if let Some(p_str) = pending_pipe_for_cancel.borrow_mut().take() {
                w.set_active_rendering_pipeline(p_str.clone().into());
                unsafe {
                    std::env::set_var("SLINT_BACKEND", &p_str);
                }
                notif_pipe_cancel.info("渲染引擎配置已更新", "新管线首选项已保存，将在下次启动客户端时自动加载生效");
                tracing::info!(target: "smagical_ui::settings", "用户选择稍后重启，渲染管线 [{}] 已暂存并在下次启动生效", p_str);
            }
        }
    });

    // -------------------------------------------------------------------------
    // 4. 窗口控制: 最小化
    // -------------------------------------------------------------------------
    // 点击右上角最小化按钮时，将主窗口最小化至系统任务栏。
    let window_weak = window.as_weak();
    let core_state_min = ctx.core_state.clone();
    window.on_minimize_window(move || {
        if let Some(w) = window_weak.upgrade() {
            core_state_min.events().dispatch(&WindowStateChangedEvent {
                state: "minimized".into(),
            });
            w.window().set_minimized(true);
        }
    });

    // -------------------------------------------------------------------------
    // 5. 窗口控制: 最大化 / 还原
    // -------------------------------------------------------------------------
    // 点击右上角最大化/还原按钮时，切换窗口最大化状态。
    let window_weak = window.as_weak();
    let core_state_max = ctx.core_state.clone();
    window.on_maximize_window(move || {
        if let Some(w) = window_weak.upgrade() {
            let is_max = w.get_is_window_maximized();
            core_state_max.events().dispatch(&WindowStateChangedEvent {
                state: if !is_max { "maximized".into() } else { "restored".into() },
            });
            w.set_is_window_maximized(!is_max);
            w.window().set_maximized(!is_max);
        }
    });

    // -------------------------------------------------------------------------
    // 5.1 窗口控制: 原生无边框窗口拖动移动 (Winit drag_window 移交 Windows DWM 接管)
    // -------------------------------------------------------------------------
    let window_weak = window.as_weak();
    window.on_start_window_drag(move || {
        if let Some(w) = window_weak.upgrade() {
            w.window().with_winit_window(|winit_window| {
                if let Err(e) = winit_window.drag_window() {
                    tracing::debug!(target: "smagical_ui::window", "Window drag failed: {:?}", e);
                }
            });
            // 拖拽过程中或结束后如果窗口最大化状态发生变动，同步 UI 状态
            let is_max = w.window().is_maximized();
            if w.get_is_window_maximized() != is_max {
                w.set_is_window_maximized(is_max);
            }
        }
    });

    // -------------------------------------------------------------------------
    // 5.2 窗口控制: 原生无边框窗口 8 方向边缘拖拽缩放 (Winit drag_resize_window)
    // -------------------------------------------------------------------------
    let window_weak_resize = window.as_weak();
    window.on_start_window_resize(move |dir_str| {
        if let Some(w) = window_weak_resize.upgrade() {
            if w.get_is_window_maximized() {
                return;
            }
            let dir = match dir_str.as_str() {
                "east" => winit::window::ResizeDirection::East,
                "west" => winit::window::ResizeDirection::West,
                "north" => winit::window::ResizeDirection::North,
                "south" => winit::window::ResizeDirection::South,
                "north-east" => winit::window::ResizeDirection::NorthEast,
                "north-west" => winit::window::ResizeDirection::NorthWest,
                "south-east" => winit::window::ResizeDirection::SouthEast,
                "south-west" => winit::window::ResizeDirection::SouthWest,
                _ => return,
            };
            w.window().with_winit_window(|winit_window| {
                if let Err(e) = winit_window.drag_resize_window(dir) {
                    tracing::debug!(target: "smagical_ui::window", "Window resize failed: {:?}", e);
                }
            });
        }
    });

    // -------------------------------------------------------------------------
    // 5.3 窗口状态自适应心跳检测 (实时同步最大化/还原状态与右上角图标)
    // -------------------------------------------------------------------------
    let window_weak_sync = window.as_weak();
    let sync_timer = Box::leak(Box::new(slint::Timer::default()));
    sync_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(150),
        move || {
            if let Some(w) = window_weak_sync.upgrade() {
                let is_max = w.window().is_maximized();
                if w.get_is_window_maximized() != is_max {
                    w.set_is_window_maximized(is_max);
                }
            }
        },
    );

    // -------------------------------------------------------------------------
    // 6. 动态更新终端字体与字号
    // -------------------------------------------------------------------------
    // 供设置面板调用：实时更换终端字体文件与字号大小，重算字形度量并标记重绘。
    let window_weak = window.as_weak();
    let renderer_clone = Rc::clone(&ctx.terminal_renderer);
    let active_terminals_clone = Rc::clone(&ctx.active_terminals);
    let core_state_font = ctx.core_state.clone();
    window.on_set_terminal_font(move |font_name_or_path, font_size| {
        if let Some(w) = window_weak.upgrade() {
            let font_str = font_name_or_path.as_str();
            let size = if font_size <= 0.0 { 14.0 } else { font_size };
            w.set_terminal_font_family(font_str.into());
            w.set_terminal_font_size(size);
            let font_for_cfg = font_str.to_string();
            let _ = core_state_font.storage().config().update(Box::new(move |c| {
                c.font_family = font_for_cfg;
                c.font_size = size;
            }));

            if let Some(ref mut renderer) = *renderer_clone.borrow_mut() {
                let font_bytes = crate::terminal::renderer::find_font_by_name(font_str);
                if let Some(bytes) = font_bytes {
                    let _ = renderer.update_font(&bytes, size);
                } else {
                    let _ = renderer.update_font_size(size);
                }
            }

            for instance in active_terminals_clone.borrow_mut().values_mut() {
                instance.parser.mark_dirty();
            }

            tracing::info!(target: "smagical_ui::settings", "动态更新终端字体: {}, 字号: {}px", font_str, size);
        }
    });

    // -------------------------------------------------------------------------
    // 7. 动态设置终端色彩调色板
    // -------------------------------------------------------------------------
    // 供设置面板调用：实时切换终端 ANSI 16 色、前景色、背景色与光标色。
    let window_weak = window.as_weak();
    let renderer_clone = Rc::clone(&ctx.terminal_renderer);
    let active_terminals_clone = Rc::clone(&ctx.active_terminals);
    window.on_set_terminal_theme(move |theme_id| {
        if let Some(_w) = window_weak.upgrade() {
            let id_str = theme_id.as_str();
            if let Some(ref mut renderer) = *renderer_clone.borrow_mut() {
                let mut palette = crate::terminal::TerminalPalette::default();
                if id_str.contains("light") {
                    palette.default_bg = [0xf6, 0xf8, 0xfa, 0xff];
                    palette.default_fg = [0x24, 0x29, 0x2f, 0xff];
                    palette.cursor_color = [0x09, 0x69, 0xda, 0xff];
                }
                renderer.update_palette(palette);
            }

            for instance in active_terminals_clone.borrow_mut().values_mut() {
                instance.parser.mark_dirty();
            }

            tracing::info!(target: "smagical_ui::settings", "动态更新终端配色方案: {}", id_str);
        }
    });

    // -------------------------------------------------------------------------
    // 8. 动态设置背景壁纸模式与图片
    // -------------------------------------------------------------------------
    // 供设置面板调用：支持 "none" (关闭) / "global" (全窗口磨砂) / "terminal" (仅终端) 三种壁纸模式。
    let window_weak = window.as_weak();
    let renderer_clone = Rc::clone(&ctx.terminal_renderer);
    let active_terminals_clone = Rc::clone(&ctx.active_terminals);
    let core_state_wp = ctx.core_state.clone();
    let wallpaper_cache_ref = Rc::clone(&ctx.wallpaper_cache);
    let wallpaper_preload_timer_ref = Rc::clone(&ctx.wallpaper_preload_timer);
    let wallpapers_ref = Rc::clone(&ctx.wallpapers);
    let active_idx_ref = Rc::clone(&ctx.active_wallpaper_idx);
    window.on_set_wallpaper(move |mode, image_path, opacity| {
        if let Some(w) = window_weak.upgrade() {
            let mode_str = mode.as_str();
            let mut path_str = image_path.as_str().to_string();
            let op = if opacity <= 0.0 { 0.20 } else { opacity.min(1.0) };

            // 若传入路径为空，自动从持久化配置或当前壁纸列表中寻找当前激活的壁纸
            if path_str.is_empty() || !std::path::Path::new(&path_str).exists() {
                if let Ok(cfg) = core_state_wp.storage().config().get() {
                    if !cfg.wallpaper_path.is_empty() && std::path::Path::new(&cfg.wallpaper_path).exists() {
                        path_str = cfg.wallpaper_path;
                    } else if !cfg.wallpaper_list.is_empty() && cfg.wallpaper_active_index < cfg.wallpaper_list.len() {
                        path_str = cfg.wallpaper_list[cfg.wallpaper_active_index].clone();
                    }
                }
            }

            w.set_wallpaper_mode(mode_str.into());
            let theme_global = w.global::<AppTheme>();
            theme_global.set_wallpaper_mode(mode_str.into());
            theme_global.set_wallpaper_opacity(op);

            // 1. 优先从内存 LRU 缓存中极速读取（0ms，不卡顿）
            let cached_img_opt = if !path_str.is_empty() && std::path::Path::new(&path_str).exists() {
                let mut cache = wallpaper_cache_ref.borrow_mut();
                if let Some(cached) = cache.get(&path_str) {
                    Some(cached.clone())
                } else if let Ok(raw_c) = crate::handlers::theme_handlers::WALLPAPER_RAW_CACHE.lock() {
                    if let Some((raw, rw, rh)) = raw_c.get(&path_str) {
                        let pixel_buffer = slint::SharedPixelBuffer::clone_from_slice(raw, *rw, *rh);
                        let loaded = slint::Image::from_rgba8(pixel_buffer);
                        if cache.len() >= 4 {
                            if let Some(oldest) = cache.keys().next().cloned() {
                                cache.remove(&oldest);
                            }
                        }
                        cache.insert(path_str.clone(), loaded.clone());
                        Some(loaded)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                Some(slint::Image::default())
            };

            if let Some(ref mut renderer) = *renderer_clone.borrow_mut() {
                match mode_str {
                    "global" => renderer.set_background_opacity(75),
                    "terminal" => renderer.set_background_opacity(70),
                    _ => renderer.set_background_opacity(100),
                }
            }

            let apply_wallpaper_ui = |w: &crate::generated::AppWindow, m_str: &str, img: slint::Image, opacity_val: f32| {
                match m_str {
                    "global" => {
                        // 平滑双缓冲淡入淡出（Cross-fade）
                        let prev_img = w.get_global_wallpaper_image();
                        if prev_img.size().width > 0 {
                            w.set_prev_wallpaper_image(prev_img);
                            w.set_wallpaper_crossfade(0.0);
                        } else {
                            w.set_wallpaper_crossfade(1.0);
                        }
                        w.set_global_wallpaper_image(img.clone());
                        w.set_global_wallpaper_opacity(opacity_val);
                        w.set_terminal_wallpaper_image(img);
                        w.set_terminal_wallpaper_opacity(opacity_val);
                    }
                    "terminal" => {
                        w.set_terminal_wallpaper_image(img);
                        w.set_terminal_wallpaper_opacity(opacity_val);
                    }
                    _ => {}
                }
            };

            // 2. 如果缓存已有，立即在 0ms 呈现；否则交给后台线程异步解码投递，主 UI 线程耗时恒等于 0ms
            if let Some(img) = cached_img_opt {
                apply_wallpaper_ui(&w, mode_str, img, op);
            } else {
                let window_weak_bg = window_weak.clone();
                let path_to_load = path_str.clone();
                let mode_bg = mode_str.to_string();

                std::thread::spawn(move || {
                    if let Some(pixel_buffer) = crate::handlers::theme_handlers::load_pixel_buffer_fast(&path_to_load) {
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(w) = window_weak_bg.upgrade() {
                                let img = slint::Image::from_rgba8(pixel_buffer);
                                match mode_bg.as_str() {
                                    "global" => {
                                        let prev_img = w.get_global_wallpaper_image();
                                        if prev_img.size().width > 0 {
                                            w.set_prev_wallpaper_image(prev_img);
                                            w.set_wallpaper_crossfade(0.0);
                                        } else {
                                            w.set_wallpaper_crossfade(1.0);
                                        }
                                        w.set_global_wallpaper_image(img.clone());
                                        w.set_global_wallpaper_opacity(op);
                                        w.set_terminal_wallpaper_image(img);
                                        w.set_terminal_wallpaper_opacity(op);
                                    }
                                    "terminal" => {
                                        w.set_terminal_wallpaper_image(img);
                                        w.set_terminal_wallpaper_opacity(op);
                                    }
                                    _ => {}
                                }
                            }
                        });
                    }
                });
            }

            // 自动触发下一张壁纸的按需后台静默预加载
            if let Ok(wps) = wallpapers_ref.try_borrow() {
                let all_imgs = crate::handlers::theme_handlers::resolve_all_wallpaper_images(&wps);
                if all_imgs.len() > 1 {
                    let cur_idx = *active_idx_ref.borrow();
                    let next_idx = (cur_idx + 1) % all_imgs.len();
                    let next_path = all_imgs[next_idx].clone();
                    crate::handlers::theme_handlers::schedule_wallpaper_preload(
                        next_path,
                        Rc::clone(&wallpaper_cache_ref),
                        Rc::clone(&wallpaper_preload_timer_ref),
                    );
                }
            }

            for instance in active_terminals_clone.borrow_mut().values_mut() {
                instance.parser.mark_dirty();
            }

            tracing::info!(target: "smagical_ui::settings", "动态更新壁纸: 模式={}, 路径={}, 不透明度={:.2}", mode_str, path_str, op);
        }

    });
}


