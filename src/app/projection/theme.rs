//! Slint 主题适配层。
//!
//! 核心主题模块只产出 `ResolvedThemePalette`，这里负责把它写入
//! `ui/theme.slint` 暴露的 `AppTheme` 全局对象。UI 组件只引用
//! `AppTheme.*` token，不直接关心主题文件、导入格式或颜色派生规则。

use slint::{Color, ComponentHandle};

use crate::app::{AppTheme, AppWindow};
use crate::theme::ResolvedThemePalette;

macro_rules! theme_token_map {
    ($apply:ident) => {
        $apply! {
            ("window-bg", set_window_bg, window_bg),
            ("topbar-bg", set_topbar_bg, topbar_bg),
            ("rail-bg", set_rail_bg, rail_bg),
            ("panel-bg", set_panel_bg, panel_bg),
            ("surface-bg", set_surface_bg, surface_bg),
            ("surface-hover", set_surface_hover, surface_hover),
            ("surface-pressed", set_surface_pressed, surface_pressed),
            ("raised-bg", set_raised_bg, raised_bg),
            ("card-bg", set_card_bg, card_bg),
            ("card-hover", set_card_hover, card_hover),
            ("inset-bg", set_inset_bg, inset_bg),
            ("dock-bg", set_dock_bg, dock_bg),
            ("terminal-bg", set_terminal_bg, terminal_bg),
            ("overlay-bg", set_overlay_bg, overlay_bg),
            ("border", set_border, border),
            ("border-soft", set_border_soft, border_soft),
            ("border-strong", set_border_strong, border_strong),
            ("border-focus", set_border_focus, border_focus),
            ("border-danger", set_border_danger, border_danger),
            ("accent", set_accent, accent),
            ("accent-blue", set_accent_blue, accent_blue),
            ("accent-violet", set_accent_violet, accent_violet),
            ("warning", set_warning, warning),
            ("danger", set_danger, danger),
            ("text", set_text, text),
            ("text-strong", set_text_strong, text_strong),
            ("text-muted", set_text_muted, text_muted),
            ("text-soft", set_text_soft, text_soft),
            ("text-secondary", set_text_secondary, text_secondary),
            ("text-disabled", set_text_disabled, text_disabled),
            ("text-inverse", set_text_inverse, text_inverse),
            ("section-text", set_section_text, section_text),
            ("status-muted", set_status_muted, status_muted),
            ("success-text", set_success_text, success_text),
            ("success-text-soft", set_success_text_soft, success_text_soft),
            ("info-text", set_info_text, info_text),
            ("danger-text", set_danger_text, danger_text),
            ("danger-text-soft", set_danger_text_soft, danger_text_soft),
            ("badge-success-bg", set_badge_success_bg, badge_success_bg),
            ("badge-info-bg", set_badge_info_bg, badge_info_bg),
            ("badge-pending-bg", set_badge_pending_bg, badge_pending_bg),
            ("badge-warning-bg", set_badge_warning_bg, badge_warning_bg),
            ("selection-bg", set_selection_bg, selection_bg),
            ("input-bg", set_input_bg, input_bg),
            ("input-bg-focus", set_input_bg_focus, input_bg_focus),
            ("input-placeholder", set_input_placeholder, input_placeholder),
            ("input-selection", set_input_selection, input_selection),
            ("button-primary-bg", set_button_primary_bg, button_primary_bg),
            ("button-primary-hover", set_button_primary_hover, button_primary_hover),
            ("button-primary-pressed", set_button_primary_pressed, button_primary_pressed),
            ("button-primary-border", set_button_primary_border, button_primary_border),
            ("button-primary-border-hover", set_button_primary_border_hover, button_primary_border_hover),
            ("button-primary-border-pressed", set_button_primary_border_pressed, button_primary_border_pressed),
            ("button-primary-text", set_button_primary_text, button_primary_text),
            ("button-secondary-bg", set_button_secondary_bg, button_secondary_bg),
            ("button-secondary-hover", set_button_secondary_hover, button_secondary_hover),
            ("button-secondary-pressed", set_button_secondary_pressed, button_secondary_pressed),
            ("button-secondary-border", set_button_secondary_border, button_secondary_border),
            ("button-secondary-text", set_button_secondary_text, button_secondary_text),
            ("button-subtle-hover", set_button_subtle_hover, button_subtle_hover),
            ("button-subtle-pressed", set_button_subtle_pressed, button_subtle_pressed),
            ("button-danger-bg", set_button_danger_bg, button_danger_bg),
            ("button-danger-hover", set_button_danger_hover, button_danger_hover),
            ("button-danger-pressed", set_button_danger_pressed, button_danger_pressed),
            ("button-danger-border", set_button_danger_border, button_danger_border),
            ("button-danger-border-hover", set_button_danger_border_hover, button_danger_border_hover),
            ("button-danger-border-pressed", set_button_danger_border_pressed, button_danger_border_pressed),
            ("button-danger-text", set_button_danger_text, button_danger_text),
            ("dialog-bg", set_dialog_bg, dialog_bg),
            ("dialog-section-bg", set_dialog_section_bg, dialog_section_bg),
            ("dialog-success-icon-bg", set_dialog_success_icon_bg, dialog_success_icon_bg),
            ("dialog-success-icon-border", set_dialog_success_icon_border, dialog_success_icon_border),
            ("dialog-danger-icon-bg", set_dialog_danger_icon_bg, dialog_danger_icon_bg),
            ("dialog-danger-icon-border", set_dialog_danger_icon_border, dialog_danger_icon_border),
            ("dialog-error-bg", set_dialog_error_bg, dialog_error_bg),
            ("dialog-error-border", set_dialog_error_border, dialog_error_border),
            ("tab-bg", set_tab_bg, tab_bg),
            ("tab-hover-bg", set_tab_hover_bg, tab_hover_bg),
            ("tab-pressed-bg", set_tab_pressed_bg, tab_pressed_bg),
            ("tab-active-bg", set_tab_active_bg, tab_active_bg),
            ("tab-active-border", set_tab_active_border, tab_active_border),
            ("tab-accent-hover", set_tab_accent_hover, tab_accent_hover),
            ("tab-close-hover-bg", set_tab_close_hover_bg, tab_close_hover_bg),
            ("tab-close-pressed-bg", set_tab_close_pressed_bg, tab_close_pressed_bg),
            ("tab-close-border-hover", set_tab_close_border_hover, tab_close_border_hover),
            ("terminal-viewport-bg", set_terminal_viewport_bg, terminal_viewport_bg),
            ("terminal-border", set_terminal_border, terminal_border),
            ("terminal-selection", set_terminal_selection, terminal_selection),
            ("topbar-border", set_topbar_border, topbar_border),
            ("terminal-text", set_terminal_text, terminal_text),
            ("terminal-muted", set_terminal_muted, terminal_muted),
        }
    };
}

pub(super) fn sync_theme_palette(window: &AppWindow, palette: ResolvedThemePalette) {
    let theme = window.global::<AppTheme>();

    macro_rules! set_theme_tokens {
        ($(($token:literal, $setter:ident, $field:ident)),+ $(,)?) => {
            $(theme.$setter(color(palette.$field));)+
        };
    }

    theme_token_map!(set_theme_tokens);
}

fn color(argb: u32) -> Color {
    Color::from_argb_u8(
        ((argb >> 24) & 0xff) as u8,
        ((argb >> 16) & 0xff) as u8,
        ((argb >> 8) & 0xff) as u8,
        (argb & 0xff) as u8,
    )
}

#[cfg(test)]
mod tests {
    macro_rules! theme_token_names {
        ($(($token:literal, $setter:ident, $field:ident)),+ $(,)?) => {
            &[$($token),+]
        };
    }

    const SYNCED_THEME_TOKENS: &[&str] = theme_token_map!(theme_token_names);
    const RUNTIME_DEFAULT_THEME_TOKENS: &[&str] = &["transparent"];

    #[test]
    fn app_theme_tokens_are_explicitly_synced_or_declared_runtime_defaults() {
        let slint_tokens = app_theme_tokens_from_slint();
        let mut known_tokens = SYNCED_THEME_TOKENS
            .iter()
            .chain(RUNTIME_DEFAULT_THEME_TOKENS.iter())
            .copied()
            .collect::<Vec<_>>();
        known_tokens.sort_unstable();

        assert_eq!(
            slint_tokens, known_tokens,
            "ui/theme.slint 新增 token 时，需要在 projection/theme.rs 中同步映射，或明确加入运行时默认白名单"
        );
    }

    fn app_theme_tokens_from_slint() -> Vec<&'static str> {
        let mut tokens = include_str!("../../../ui/theme.slint")
            .lines()
            .filter_map(|line| {
                line.trim()
                    .strip_prefix("in-out property <color> ")
                    .and_then(|rest| rest.split_once(':'))
                    .map(|(token, _)| token.trim())
            })
            .collect::<Vec<_>>();
        tokens.sort_unstable();
        tokens
    }
}
