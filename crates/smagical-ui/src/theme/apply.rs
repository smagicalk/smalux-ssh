use slint::{Brush, Color, ComponentHandle};
use smagical_core::theme::{ColorScheme, ResolvedUiTheme, ThemeError, ThemeService};

use crate::{AppColorScheme, AppTheme, AppWindow};

/// 将已解析 UI 主题写入当前窗口的 `AppTheme` global。
pub fn apply_ui_theme(window: &AppWindow, theme: &ResolvedUiTheme) {
    let global = window.global::<AppTheme>();
    global.set_theme_id(theme.metadata.id.as_ref().into());
    let system = theme.tokens.color_scheme == ColorScheme::System;
    global.set_use_system_palette(system);
    global.set_color_scheme(match theme.tokens.color_scheme {
        ColorScheme::System => AppColorScheme::System,
        ColorScheme::Light => AppColorScheme::Light,
        ColorScheme::Dark => AppColorScheme::Dark,
    });

    macro_rules! set_color {
        ($setter:ident, $value:expr) => {
            global.$setter(Brush::from(parse_color($value)));
        };
    }
    let colors = &theme.tokens;
    set_color!(set_custom_window_background, &colors.window_background);
    set_color!(set_custom_panel_background, &colors.panel_background);
    set_color!(set_custom_surface_background, &colors.surface_background);
    set_color!(set_custom_control_background, &colors.control_background);
    set_color!(set_custom_foreground, &colors.foreground);
    set_color!(
        set_custom_secondary_foreground,
        &colors.secondary_foreground
    );
    set_color!(set_custom_disabled_foreground, &colors.disabled_foreground);
    set_color!(set_custom_accent, &colors.accent);
    set_color!(set_custom_hover_background, &colors.hover_background);
    set_color!(set_custom_pressed_background, &colors.pressed_background);
    set_color!(set_custom_selected_background, &colors.selected_background);
    set_color!(set_custom_selected_foreground, &colors.selected_foreground);
    set_color!(set_custom_border, &colors.border);
    set_color!(set_custom_focus_border, &colors.focus_border);
    set_color!(set_custom_success, &colors.success);
    set_color!(set_custom_warning, &colors.warning);
    set_color!(set_custom_danger, &colors.danger);
    set_color!(set_custom_info, &colors.info);

    let metrics = &theme.metrics;
    global.set_radius_small(metrics.radius_small);
    global.set_radius_medium(metrics.radius_medium);
    global.set_radius_large(metrics.radius_large);
    global.set_spacing_small(metrics.spacing_small);
    global.set_spacing_medium(metrics.spacing_medium);
    global.set_spacing_large(metrics.spacing_large);
    global.set_border_width(metrics.border_width);
    global.set_control_height(metrics.control_height);
    global.set_icon_size(metrics.icon_size);
}

/// 按稳定 ID 解析并应用主题。该函数不写配置或主题文件。
pub fn apply_theme_by_id(
    window: &AppWindow,
    service: &ThemeService,
    theme_id: impl AsRef<str>,
) -> Result<(), ThemeError> {
    let theme = service.resolve_ui(theme_id)?;
    apply_ui_theme(window, &theme);
    Ok(())
}

/// 恢复 Slint `Palette` 驱动的系统主题。
pub fn restore_system_theme(window: &AppWindow, service: &ThemeService) -> Result<(), ThemeError> {
    apply_theme_by_id(window, service, "builtin.ui.system")
}

fn parse_color(value: &str) -> Color {
    let hex = value.trim_start_matches('#');
    let channel =
        |start| u8::from_str_radix(&hex[start..start + 2], 16).expect("颜色已由核心层验证");
    let alpha = if hex.len() == 8 { channel(6) } else { 255 };
    Color::from_argb_u8(alpha, channel(0), channel(2), channel(4))
}
