//! 窗口级通用交互（主题切换、深浅色模式、系统窗口最小化/最大化/关闭）回调绑定。
//!
//! 提供跨平台通用桌面窗口状态管理与主题系统的事件响应。

use std::rc::Rc;
use slint::ComponentHandle;
use theme::apply_theme_by_id;

use crate::generated::AppWindow;
use crate::handlers::AppContext;

use crate::theme;

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

            match apply_theme_by_id(&w, &themes_clone, normalized_id) {
                Ok(()) => {
                    w.set_current_theme_name(name.into());
                    let is_light = normalized_id.contains("light") || normalized_id.contains("dawn") || normalized_id.contains("latte");
                    w.set_is_dark_mode(!is_light);
                    core_state_theme.app_hooks().dispatch_theme_changed(normalized_id, !is_light);
                    core_state_theme.app_hooks().dispatch_config_changed(&smagical_core::ConfigChangeEvent::new(
                        "appearance.theme",
                        "",
                        normalized_id,
                        "switch_theme",
                    ));
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
                let _ = apply_theme_by_id(&w, &themes_clone, "builtin.ui.darcula");
                w.set_current_theme_name("Darcula".into());
                w.set_is_dark_mode(true);
            } else {
                let _ = apply_theme_by_id(&w, &themes_clone, "builtin.ui.github-light");
                w.set_current_theme_name("GitHub Light".into());
                w.set_is_dark_mode(false);
            }

            core_state_color_mode.app_hooks().dispatch_theme_mode_toggled(next_dark);
            core_state_color_mode.app_hooks().dispatch_config_changed(&smagical_core::ConfigChangeEvent::new(
                "appearance.color_mode",
                if is_dark { "dark" } else { "light" },
                if next_dark { "dark" } else { "light" },
                "toggle_color_mode",
            ));

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
                w.set_debug_logs(slint::ModelRc::default());
                if w.get_active_left_tab() == "debug" {
                    w.set_active_left_tab("hosts".into());
                }
            } else {
                crate::debug_ui::sync_ui_debug_logs(&w);
            }
            core_state_debug.app_hooks().dispatch_config_changed(&smagical_core::ConfigChangeEvent::new(
                "developer.debug_enabled",
                if enabled { "false" } else { "true" },
                if enabled { "true" } else { "false" },
                "toggle_debug_enabled",
            ));
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


    // -------------------------------------------------------------------------
    // 3. 窗口控制: 关闭应用 (带安全守护与退出归档 Hook)

    // -------------------------------------------------------------------------
    // 点击右上角红色关闭按钮时，触发 before_exit 询问与 exit 终态归档，安全退出客户端进程。
    let core_state_close = ctx.core_state.clone();
    let pane_groups_close = Rc::clone(&ctx.pane_groups);
    let persistence_guard_close = std::sync::Arc::clone(&ctx.persistence_guard);
    window.on_close_window(move || {
        let active_count = pane_groups_close.borrow().iter().map(|g| g.tabs.len()).sum();
        let exit_ctx = smagical_core::AppExitContext::normal(active_count);
        let decision = core_state_close.app_hooks().dispatch_app_before_exit(&exit_ctx);
        if matches!(decision, smagical_core::HookDecision::Abort { .. }) {
            tracing::warn!(target: "smagical_ui::window", "应用退出流程被安全守护插件拦截");
            return;
        }
        // 等待所有后台会话历史持久化写盘完成，杜绝数据丢失
        persistence_guard_close.flush_and_wait(std::time::Duration::from_millis(1000));
        core_state_close.app_hooks().dispatch_app_exit(&exit_ctx);
        std::process::exit(0);
    });


    // -------------------------------------------------------------------------
    // 4. 窗口控制: 最小化
    // -------------------------------------------------------------------------
    // 点击右上角最小化按钮时，将主窗口最小化至系统任务栏。
    let window_weak = window.as_weak();
    let core_state_min = ctx.core_state.clone();
    window.on_minimize_window(move || {
        if let Some(w) = window_weak.upgrade() {
            core_state_min.app_hooks().dispatch_shell_window_state_changed(smagical_core::WindowState::Minimized);
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
            core_state_max.app_hooks().dispatch_shell_window_state_changed(if !is_max {
                smagical_core::WindowState::Maximized
            } else {
                smagical_core::WindowState::Normal
            });
            w.set_is_window_maximized(!is_max);
            w.window().set_maximized(!is_max);
        }

    });

    // -------------------------------------------------------------------------
    // 6. 动态更新终端字体与字号
    // -------------------------------------------------------------------------
    // 供设置面板调用：实时更换终端字体文件与字号大小，重算字形度量并标记重绘。
    let window_weak = window.as_weak();
    let renderer_clone = Rc::clone(&ctx.terminal_renderer);
    let active_terminals_clone = Rc::clone(&ctx.active_terminals);
    window.on_set_terminal_font(move |font_name_or_path, font_size| {
        if let Some(w) = window_weak.upgrade() {
            let font_str = font_name_or_path.as_str();
            let size = if font_size <= 0.0 { 14.0 } else { font_size };
            w.set_terminal_font_family(font_str.into());
            w.set_terminal_font_size(size);

            if let Some(ref mut renderer) = *renderer_clone.borrow_mut() {
                let font_bytes = if std::path::Path::new(font_str).exists() {
                    std::fs::read(font_str).ok()
                } else {
                    None
                };

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
    window.on_set_wallpaper(move |mode, image_path, opacity| {
        if let Some(w) = window_weak.upgrade() {
            let mode_str = mode.as_str();
            let path_str = image_path.as_str();
            let op = if opacity <= 0.0 { 0.20 } else { opacity.min(1.0) };

            w.set_wallpaper_mode(mode_str.into());

            let img = if !path_str.is_empty() && std::path::Path::new(path_str).exists() {
                slint::Image::load_from_path(std::path::Path::new(path_str)).unwrap_or_default()
            } else {
                slint::Image::default()
            };

            match mode_str {
                "global" => {
                    w.set_global_wallpaper_image(img);
                    w.set_global_wallpaper_opacity(op);
                    if let Some(ref mut renderer) = *renderer_clone.borrow_mut() {
                        renderer.set_background_opacity(85); // 适度半透明透底
                    }
                }
                "terminal" => {
                    w.set_terminal_wallpaper_image(img);
                    w.set_terminal_wallpaper_opacity(op);
                    if let Some(ref mut renderer) = *renderer_clone.borrow_mut() {
                        renderer.set_background_opacity(70); // 终端视口透底
                    }
                }
                _ => {
                    if let Some(ref mut renderer) = *renderer_clone.borrow_mut() {
                        renderer.set_background_opacity(100); // 纯色不透明
                    }
                }
            }

            for instance in active_terminals_clone.borrow_mut().values_mut() {
                instance.parser.mark_dirty();
            }

            tracing::info!(target: "smagical_ui::settings", "动态更新壁纸: 模式={}, 路径={}, 不透明度={:.2}", mode_str, path_str, op);
        }

    });
}


