//! 窗口级通用交互（主题切换、深浅色模式、系统窗口最小化/最大化/关闭）回调绑定。
//!
//! 提供跨平台通用桌面窗口状态管理与主题系统的事件响应。

use std::rc::Rc;
use slint::ComponentHandle;
use theme::apply_theme_by_id;

use crate::debug_ui::sync_ui_debug_logs;
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
    window.on_switch_theme(move |theme_id| {
        if let Some(w) = window_weak.upgrade() {
            let id_str = theme_id.as_str();
            let _ = apply_theme_by_id(&w, &themes_clone, id_str);
            let name = if id_str.contains("darcula") {
                "Darcula"
            } else if id_str.contains("monokai") {
                "Monokai"
            } else if id_str.contains("onedark") || id_str.contains("one-dark") {
                "One Dark"
            } else if id_str.contains("solarized-light") {
                "Solarized Light"
            } else if id_str.contains("solarized") {
                "Solarized"
            } else if id_str.contains("github-light") || id_str.contains("light") {
                "GitHub Light"
            } else if id_str.contains("github-dark") {
                "GitHub Dark"
            } else {
                "System"
            };
            w.set_current_theme_name(name.into());

            // 依据主题标识自动同步深浅色布尔标志位
            let is_light = id_str.contains("light") || id_str.contains("dawn") || id_str.contains("latte");
            w.set_is_dark_mode(!is_light);

            tracing::info!(target: "smagical_ui::theme", "切换应用配色主题: {} ({})", name, id_str);
            sync_ui_debug_logs(&w);
        }
    });

    // -------------------------------------------------------------------------
    // 2. 深色 / 浅色模式一键切换回调
    // -------------------------------------------------------------------------
    // 点击顶部栏或设置中的深浅色切换图标时触发，快速在默认深色 (Darcula) 与默认浅色 (GitHub Light) 之间翻转。
    let window_weak = window.as_weak();
    let themes_clone = Rc::clone(&ctx.themes);
    window.on_toggle_color_mode(move || {
        if let Some(w) = window_weak.upgrade() {
            let is_dark = w.get_is_dark_mode();
            let next_dark = !is_dark;
            w.set_is_dark_mode(next_dark);

            if next_dark {
                let _ = apply_theme_by_id(&w, &themes_clone, "builtin.ui.darcula");
                w.set_current_theme_name("Darcula".into());
            } else {
                let _ = apply_theme_by_id(&w, &themes_clone, "builtin.ui.github-light");
                w.set_current_theme_name("GitHub Light".into());
            }

            tracing::info!(target: "smagical_ui::theme", "{}", if next_dark { "切换至深色模式 (Darcula)" } else { "切换至浅色模式 (GitHub Light)" });
            sync_ui_debug_logs(&w);
        }
    });

    // -------------------------------------------------------------------------
    // 3. 窗口控制: 关闭应用
    // -------------------------------------------------------------------------
    // 点击右上角红色关闭按钮时，安全退出整个客户端进程。
    window.on_close_window(|| {
        std::process::exit(0);
    });

    // -------------------------------------------------------------------------
    // 4. 窗口控制: 最小化
    // -------------------------------------------------------------------------
    // 点击右上角最小化按钮时，将主窗口最小化至系统任务栏。
    let window_weak = window.as_weak();
    window.on_minimize_window(move || {
        if let Some(w) = window_weak.upgrade() {
            w.window().set_minimized(true);
        }
    });

    // -------------------------------------------------------------------------
    // 5. 窗口控制: 最大化 / 还原
    // -------------------------------------------------------------------------
    // 双击标题栏或点击最大化/还原按钮时，在全屏最大化与窗口还原尺寸之间切换。
    let window_weak = window.as_weak();
    window.on_maximize_window(move || {
        if let Some(w) = window_weak.upgrade() {
            let cur = w.get_is_window_maximized();
            w.window().set_maximized(!cur);
            w.set_is_window_maximized(!cur);
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


