//! 主题文档到 UI 运行时色板的解析。

use super::color::parse_color;
use super::{ResolvedThemePalette, ThemeDocument, ThemeError};

impl ThemeDocument {
    /// 校验原生主题文档。
    pub fn validate(&self) -> Result<(), ThemeError> {
        if self.schema_version != 1 {
            return Err(ThemeError::UnsupportedSchema(self.schema_version));
        }
        if self.id.trim().is_empty() {
            return Err(ThemeError::MissingId);
        }
        self.resolve_palette().map(|_| ())
    }

    /// 将结构化主题解析成 Slint 直接消费的扁平 UI token。
    pub fn resolve_palette(&self) -> Result<ResolvedThemePalette, ThemeError> {
        Ok(ResolvedThemePalette {
            window_bg: parse_color("frame.window", &self.frame.window)?,
            topbar_bg: parse_color("frame.topbar", &self.frame.topbar)?,
            rail_bg: parse_color("frame.rail", &self.frame.rail)?,
            panel_bg: parse_color("frame.left_sidebar", &self.frame.left_sidebar)?,
            surface_bg: parse_color("workspace.surface", &self.workspace.surface)?,
            surface_hover: parse_color("workspace.surface_hover", &self.workspace.surface_hover)?,
            surface_pressed: parse_color("workspace.panel", &self.workspace.panel)?,
            raised_bg: parse_color("workspace.surface_raised", &self.workspace.surface_raised)?,
            card_bg: parse_color("workspace.card", &self.workspace.card)?,
            card_hover: parse_color("workspace.card_hover", &self.workspace.card_hover)?,
            inset_bg: parse_color("workspace.background", &self.workspace.background)?,
            dock_bg: parse_color("frame.overlay", &self.frame.overlay)?,
            terminal_bg: parse_color("terminal.background", &self.terminal.background)?,
            overlay_bg: parse_color("frame.overlay", &self.frame.overlay)?,
            border: parse_color("border.normal", &self.border.normal)?,
            border_soft: parse_color("border.soft", &self.border.soft)?,
            border_strong: parse_color("border.strong", &self.border.strong)?,
            border_focus: parse_color("border.focus", &self.border.focus)?,
            border_danger: parse_color("border.danger", &self.border.danger)?,
            accent: parse_color("state.success", &self.state.success)?,
            accent_blue: parse_color("state.info", &self.state.info)?,
            accent_violet: parse_color("state.pending", &self.state.pending)?,
            warning: parse_color("state.warning", &self.state.warning)?,
            danger: parse_color("state.danger", &self.state.danger)?,
            text: parse_color("text.normal", &self.text.normal)?,
            text_strong: parse_color("text.strong", &self.text.strong)?,
            text_muted: parse_color("text.muted", &self.text.muted)?,
            text_soft: parse_color("text.soft", &self.text.soft)?,
            text_secondary: parse_color("text.soft", &self.text.soft)?,
            text_disabled: parse_color("text.disabled", &self.text.disabled)?,
            text_inverse: parse_color("text.inverse", &self.text.inverse)?,
            section_text: parse_color("text.disabled", &self.text.disabled)?,
            status_muted: parse_color("text.soft", &self.text.soft)?,
            success_text: parse_color("state.success", &self.state.success)?,
            success_text_soft: parse_color("terminal.foreground", &self.terminal.foreground)?,
            info_text: parse_color("state.info", &self.state.info)?,
            danger_text: parse_color("state.danger", &self.state.danger)?,
            danger_text_soft: parse_color(
                "terminal.ansi.bright_red",
                &self.terminal.ansi.bright_red,
            )?,
            badge_success_bg: parse_color(
                "badge.success_background",
                &self.badge.success_background,
            )?,
            badge_info_bg: parse_color("badge.info_background", &self.badge.info_background)?,
            badge_pending_bg: parse_color(
                "badge.pending_background",
                &self.badge.pending_background,
            )?,
            badge_warning_bg: parse_color(
                "badge.warning_background",
                &self.badge.warning_background,
            )?,
            selection_bg: parse_color("terminal.selection", &self.terminal.selection)?,
            input_bg: parse_color("input.background", &self.input.background)?,
            input_bg_focus: parse_color("input.background_focus", &self.input.background_focus)?,
            input_placeholder: parse_color("input.placeholder", &self.input.placeholder)?,
            input_selection: parse_color("input.selection", &self.input.selection)?,
            button_primary_bg: parse_color(
                "button.primary.background",
                &self.button.primary.background,
            )?,
            button_primary_hover: parse_color(
                "button.primary.background_hover",
                &self.button.primary.background_hover,
            )?,
            button_primary_pressed: parse_color("workspace.panel", &self.workspace.panel)?,
            button_primary_border: parse_color(
                "button.primary.border",
                &self.button.primary.border,
            )?,
            button_primary_border_hover: parse_color("border.focus", &self.border.focus)?,
            button_primary_border_pressed: parse_color("border.strong", &self.border.strong)?,
            button_primary_text: parse_color("button.primary.text", &self.button.primary.text)?,
            button_secondary_bg: parse_color(
                "button.secondary.background",
                &self.button.secondary.background,
            )?,
            button_secondary_hover: parse_color(
                "button.secondary.background_hover",
                &self.button.secondary.background_hover,
            )?,
            button_secondary_pressed: parse_color(
                "workspace.background",
                &self.workspace.background,
            )?,
            button_secondary_border: parse_color(
                "button.secondary.border",
                &self.button.secondary.border,
            )?,
            button_secondary_text: parse_color(
                "button.secondary.text",
                &self.button.secondary.text,
            )?,
            button_subtle_hover: parse_color(
                "workspace.surface_hover",
                &self.workspace.surface_hover,
            )?,
            button_subtle_pressed: parse_color("workspace.surface", &self.workspace.surface)?,
            button_danger_bg: parse_color(
                "dialog.header_icon_background",
                &self.dialog.header_icon_background,
            )?,
            button_danger_hover: parse_color("dialog.background", &self.dialog.background)?,
            button_danger_pressed: parse_color("state.danger", &self.state.danger)?,
            button_danger_border: parse_color("border.danger", &self.border.danger)?,
            button_danger_border_hover: parse_color("state.danger", &self.state.danger)?,
            button_danger_border_pressed: parse_color(
                "terminal.ansi.bright_red",
                &self.terminal.ansi.bright_red,
            )?,
            button_danger_text: parse_color("state.danger", &self.state.danger)?,
            dialog_bg: parse_color("dialog.background", &self.dialog.background)?,
            dialog_section_bg: parse_color("workspace.card", &self.workspace.card)?,
            dialog_success_icon_bg: parse_color(
                "workspace.card_hover",
                &self.workspace.card_hover,
            )?,
            dialog_success_icon_border: parse_color("state.success", &self.state.success)?,
            dialog_danger_icon_bg: parse_color(
                "dialog.header_icon_background",
                &self.dialog.header_icon_background,
            )?,
            dialog_danger_icon_border: parse_color(
                "dialog.header_icon_border",
                &self.dialog.header_icon_border,
            )?,
            dialog_error_bg: parse_color(
                "dialog.header_icon_background",
                &self.dialog.header_icon_background,
            )?,
            dialog_error_border: parse_color("border.danger", &self.border.danger)?,
            tab_bg: parse_color("tabs.background", &self.tabs.background)?,
            tab_hover_bg: parse_color("workspace.surface_hover", &self.workspace.surface_hover)?,
            tab_pressed_bg: parse_color("workspace.background", &self.workspace.background)?,
            tab_active_bg: parse_color("tabs.active_background", &self.tabs.active_background)?,
            tab_active_border: parse_color("tabs.active_border", &self.tabs.active_border)?,
            tab_accent_hover: parse_color("workspace.card_hover", &self.workspace.card_hover)?,
            tab_close_hover_bg: parse_color("workspace.card_hover", &self.workspace.card_hover)?,
            tab_close_pressed_bg: parse_color(
                "dialog.header_icon_background",
                &self.dialog.header_icon_background,
            )?,
            tab_close_border_hover: parse_color("border.strong", &self.border.strong)?,
            terminal_viewport_bg: parse_color("terminal.background", &self.terminal.background)?,
            terminal_border: parse_color("border.soft", &self.border.soft)?,
            terminal_selection: parse_color("terminal.selection", &self.terminal.selection)?,
            topbar_border: parse_color("frame.splitter", &self.frame.splitter)?,
            terminal_text: parse_color("terminal.foreground", &self.terminal.foreground)?,
            terminal_muted: parse_color("terminal.muted", &self.terminal.muted)?,
        })
    }
}
