//! 可导入导出的 UI 主题配置。

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

use crate::model::BuiltInTheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedThemePalette {
    pub window_bg: u32,
    pub topbar_bg: u32,
    pub rail_bg: u32,
    pub panel_bg: u32,
    pub surface_bg: u32,
    pub surface_hover: u32,
    pub surface_pressed: u32,
    pub raised_bg: u32,
    pub card_bg: u32,
    pub card_hover: u32,
    pub inset_bg: u32,
    pub dock_bg: u32,
    pub terminal_bg: u32,
    pub overlay_bg: u32,
    pub border: u32,
    pub border_soft: u32,
    pub border_strong: u32,
    pub border_focus: u32,
    pub border_danger: u32,
    pub accent: u32,
    pub accent_blue: u32,
    pub accent_violet: u32,
    pub warning: u32,
    pub danger: u32,
    pub text: u32,
    pub text_strong: u32,
    pub text_muted: u32,
    pub text_soft: u32,
    pub text_secondary: u32,
    pub text_disabled: u32,
    pub text_inverse: u32,
    pub section_text: u32,
    pub status_muted: u32,
    pub success_text: u32,
    pub success_text_soft: u32,
    pub info_text: u32,
    pub danger_text: u32,
    pub danger_text_soft: u32,
    pub badge_success_bg: u32,
    pub badge_info_bg: u32,
    pub badge_pending_bg: u32,
    pub badge_warning_bg: u32,
    pub selection_bg: u32,
    pub input_bg: u32,
    pub input_bg_focus: u32,
    pub input_placeholder: u32,
    pub input_selection: u32,
    pub button_primary_bg: u32,
    pub button_primary_hover: u32,
    pub button_primary_pressed: u32,
    pub button_primary_border: u32,
    pub button_primary_border_hover: u32,
    pub button_primary_border_pressed: u32,
    pub button_primary_text: u32,
    pub button_secondary_bg: u32,
    pub button_secondary_hover: u32,
    pub button_secondary_pressed: u32,
    pub button_secondary_border: u32,
    pub button_secondary_text: u32,
    pub button_subtle_hover: u32,
    pub button_subtle_pressed: u32,
    pub button_danger_bg: u32,
    pub button_danger_hover: u32,
    pub button_danger_pressed: u32,
    pub button_danger_border: u32,
    pub button_danger_border_hover: u32,
    pub button_danger_border_pressed: u32,
    pub button_danger_text: u32,
    pub dialog_bg: u32,
    pub dialog_section_bg: u32,
    pub dialog_success_icon_bg: u32,
    pub dialog_success_icon_border: u32,
    pub dialog_danger_icon_bg: u32,
    pub dialog_danger_icon_border: u32,
    pub dialog_error_bg: u32,
    pub dialog_error_border: u32,
    pub tab_bg: u32,
    pub tab_hover_bg: u32,
    pub tab_pressed_bg: u32,
    pub tab_active_bg: u32,
    pub tab_active_border: u32,
    pub tab_accent_hover: u32,
    pub tab_close_hover_bg: u32,
    pub tab_close_pressed_bg: u32,
    pub tab_close_border_hover: u32,
    pub terminal_viewport_bg: u32,
    pub terminal_border: u32,
    pub terminal_selection: u32,
    pub topbar_border: u32,
    pub terminal_text: u32,
    pub terminal_muted: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeDocument {
    pub schema_version: u16,
    pub id: String,
    pub name: String,
    pub kind: ThemeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,
    #[serde(default)]
    pub meta: ThemeMeta,
    #[serde(default)]
    pub font: ThemeFonts,
    pub frame: FrameColors,
    pub workspace: WorkspaceColors,
    pub border: BorderColors,
    pub text: TextColors,
    pub state: StateColors,
    pub input: InputColors,
    pub button: ButtonColors,
    pub dialog: DialogColors,
    #[serde(default)]
    pub badge: BadgeColors,
    pub tabs: TabColors,
    pub terminal: TerminalColors,
    #[serde(default)]
    pub background: BackgroundColors,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeKind {
    Dark,
    Light,
    HighContrast,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ThemeMeta {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub author: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub license: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ThemeFonts {
    #[serde(default)]
    pub ui: FontSpec,
    #[serde(default)]
    pub terminal: TerminalFontSpec,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FontSpec {
    pub family: String,
    pub size: u16,
}

impl Default for FontSpec {
    fn default() -> Self {
        Self {
            family: "Inter".to_owned(),
            size: 12,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalFontSpec {
    pub family: String,
    pub size: u16,
    pub line_height_percent: u16,
}

impl Default for TerminalFontSpec {
    fn default() -> Self {
        Self {
            family: "JetBrains Mono".to_owned(),
            size: 14,
            line_height_percent: 120,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FrameColors {
    pub window: String,
    pub titlebar: String,
    pub topbar: String,
    pub rail: String,
    pub left_sidebar: String,
    pub right_sidebar: String,
    pub splitter: String,
    pub overlay: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceColors {
    pub background: String,
    pub panel: String,
    pub surface: String,
    pub surface_hover: String,
    pub surface_raised: String,
    pub card: String,
    pub card_hover: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BorderColors {
    pub normal: String,
    pub soft: String,
    pub strong: String,
    pub focus: String,
    pub danger: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextColors {
    pub normal: String,
    pub strong: String,
    pub muted: String,
    pub soft: String,
    pub disabled: String,
    pub inverse: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateColors {
    pub success: String,
    pub info: String,
    pub warning: String,
    pub danger: String,
    pub pending: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputColors {
    pub background: String,
    pub background_focus: String,
    pub border: String,
    pub border_focus: String,
    pub text: String,
    pub placeholder: String,
    pub selection: String,
    pub cursor: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ButtonColors {
    pub primary: ButtonVariantColors,
    pub secondary: ButtonVariantColors,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ButtonVariantColors {
    pub background: String,
    pub background_hover: String,
    pub border: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogColors {
    pub background: String,
    pub header_icon_background: String,
    pub header_icon_border: String,
    pub footer_border: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BadgeColors {
    pub success_background: String,
    pub info_background: String,
    pub pending_background: String,
    pub warning_background: String,
}

impl Default for BadgeColors {
    fn default() -> Self {
        Self {
            success_background: "#15352f".to_owned(),
            info_background: "#142a36".to_owned(),
            pending_background: "#291f3c".to_owned(),
            warning_background: "#2b2313".to_owned(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabColors {
    pub background: String,
    pub active_background: String,
    pub active_border: String,
    pub text: String,
    pub muted: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalColors {
    pub background: String,
    pub foreground: String,
    pub muted: String,
    pub cursor: String,
    pub selection: String,
    pub selection_text: String,
    pub ansi: AnsiColors,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnsiColors {
    pub black: String,
    pub red: String,
    pub green: String,
    pub yellow: String,
    pub blue: String,
    pub magenta: String,
    pub cyan: String,
    pub white: String,
    pub bright_black: String,
    pub bright_red: String,
    pub bright_green: String,
    pub bright_yellow: String,
    pub bright_blue: String,
    pub bright_magenta: String,
    pub bright_cyan: String,
    pub bright_white: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackgroundColors {
    pub color: String,
    pub image_dim: String,
    pub image_overlay: String,
}

impl Default for BackgroundColors {
    fn default() -> Self {
        Self {
            color: "#080d12".to_owned(),
            image_dim: "#00000080".to_owned(),
            image_overlay: "#080d1266".to_owned(),
        }
    }
}

#[derive(Debug, Error)]
pub enum ThemeError {
    #[error("主题 TOML 无效：{0}")]
    InvalidToml(#[from] toml::de::Error),
    #[error("主题 JSON 无效：{0}")]
    InvalidJson(serde_json::Error),
    #[error("主题导出失败：{0}")]
    ExportToml(#[from] toml::ser::Error),
    #[error("主题 JSON 导出失败：{0}")]
    ExportJson(#[from] serde_json::Error),
    #[error("主题 schema_version 不支持：{0}")]
    UnsupportedSchema(u16),
    #[error("暂不支持该主题格式：{0}")]
    UnsupportedFormat(&'static str),
    #[error("外部主题结构无效：{0}")]
    InvalidExternalTheme(&'static str),
    #[error("主题 id 不能为空")]
    MissingId,
    #[error("主题颜色 {field} 无效：{value}")]
    InvalidColor { field: &'static str, value: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeExchangeFormat {
    NativeToml,
    VsCodeJson,
    WindowsTerminalJson,
    AlacrittyToml,
    ItermColors,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportedTheme {
    pub file_name: String,
    pub content: String,
    pub report: ThemeExchangeReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeExchangeReport {
    pub format: ThemeExchangeFormat,
    pub lossy: bool,
    pub warnings: Vec<String>,
}

impl ThemeDocument {
    pub fn from_toml(input: &str) -> Result<Self, ThemeError> {
        let document: Self = toml::from_str(input)?;
        document.validate()?;
        Ok(document)
    }

    pub fn from_import(input: &str, source_name: &str) -> Result<Self, ThemeError> {
        match import_extension(source_name) {
            Some("json") => theme_document_from_json(input, source_name),
            Some("toml") => match Self::from_toml(input) {
                Ok(document) => Ok(document),
                Err(_) if looks_like_alacritty_toml(input) => {
                    theme_document_from_alacritty_toml(input, source_name)
                }
                Err(error) => Err(error),
            },
            Some("itermcolors") => Err(ThemeError::UnsupportedFormat(
                "iTerm2 .itermcolors 转换器尚未接入",
            )),
            _ if looks_like_json(input) => theme_document_from_json(input, source_name),
            _ if looks_like_alacritty_toml(input) => {
                theme_document_from_alacritty_toml(input, source_name)
            }
            _ => Self::from_toml(input),
        }
    }

    pub fn to_toml(&self) -> Result<String, ThemeError> {
        Ok(toml::to_string_pretty(self)?)
    }

    pub fn export(&self, format: ThemeExchangeFormat) -> Result<ExportedTheme, ThemeError> {
        match format {
            ThemeExchangeFormat::NativeToml => Ok(ExportedTheme {
                file_name: format!("{}.smagical-theme.toml", self.id),
                content: self.to_toml()?,
                report: ThemeExchangeReport {
                    format,
                    lossy: false,
                    warnings: Vec::new(),
                },
            }),
            ThemeExchangeFormat::VsCodeJson => Ok(ExportedTheme {
                file_name: format!("{}-vscode-color-theme.json", self.id),
                content: self.export_vscode_json()?,
                report: ThemeExchangeReport {
                    format,
                    lossy: true,
                    warnings: vec![
                        "VSCode tokenColors/semanticTokenColors 不从 SSH UI 主题生成".to_owned(),
                    ],
                },
            }),
            ThemeExchangeFormat::WindowsTerminalJson => Ok(ExportedTheme {
                file_name: format!("{}-windows-terminal.json", self.id),
                content: self.export_windows_terminal_json()?,
                report: ThemeExchangeReport {
                    format,
                    lossy: false,
                    warnings: Vec::new(),
                },
            }),
            ThemeExchangeFormat::AlacrittyToml => Ok(ExportedTheme {
                file_name: format!("{}-alacritty.toml", self.id),
                content: self.export_alacritty_toml(),
                report: ThemeExchangeReport {
                    format,
                    lossy: true,
                    warnings: vec!["Alacritty 只导出终端字体和终端颜色块".to_owned()],
                },
            }),
            ThemeExchangeFormat::ItermColors => Err(ThemeError::UnsupportedFormat(
                "iTerm2 .itermcolors 转换器尚未接入",
            )),
        }
    }

    pub fn validate(&self) -> Result<(), ThemeError> {
        if self.schema_version != 1 {
            return Err(ThemeError::UnsupportedSchema(self.schema_version));
        }
        if self.id.trim().is_empty() {
            return Err(ThemeError::MissingId);
        }
        self.resolve_palette().map(|_| ())
    }

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

    fn export_vscode_json(&self) -> Result<String, ThemeError> {
        let theme = json!({
            "$schema": "vscode://schemas/color-theme",
            "name": self.name,
            "type": match self.kind {
                ThemeKind::Dark => "dark",
                ThemeKind::Light => "light",
                ThemeKind::HighContrast => "hc",
            },
            "colors": {
                "foreground": self.text.normal,
                "focusBorder": self.border.focus,
                "titleBar.activeBackground": self.frame.titlebar,
                "titleBar.activeForeground": self.text.strong,
                "activityBar.background": self.frame.rail,
                "activityBar.foreground": self.text.normal,
                "sideBar.background": self.frame.left_sidebar,
                "sideBar.foreground": self.text.normal,
                "panel.background": self.workspace.panel,
                "editor.background": self.terminal.background,
                "editor.foreground": self.terminal.foreground,
                "editor.selectionBackground": self.terminal.selection,
                "terminal.background": self.terminal.background,
                "terminal.foreground": self.terminal.foreground,
                "terminal.ansiBlack": self.terminal.ansi.black,
                "terminal.ansiRed": self.terminal.ansi.red,
                "terminal.ansiGreen": self.terminal.ansi.green,
                "terminal.ansiYellow": self.terminal.ansi.yellow,
                "terminal.ansiBlue": self.terminal.ansi.blue,
                "terminal.ansiMagenta": self.terminal.ansi.magenta,
                "terminal.ansiCyan": self.terminal.ansi.cyan,
                "terminal.ansiWhite": self.terminal.ansi.white,
                "terminal.ansiBrightBlack": self.terminal.ansi.bright_black,
                "terminal.ansiBrightRed": self.terminal.ansi.bright_red,
                "terminal.ansiBrightGreen": self.terminal.ansi.bright_green,
                "terminal.ansiBrightYellow": self.terminal.ansi.bright_yellow,
                "terminal.ansiBrightBlue": self.terminal.ansi.bright_blue,
                "terminal.ansiBrightMagenta": self.terminal.ansi.bright_magenta,
                "terminal.ansiBrightCyan": self.terminal.ansi.bright_cyan,
                "terminal.ansiBrightWhite": self.terminal.ansi.bright_white,
                "button.background": self.button.primary.background,
                "button.foreground": self.button.primary.text,
                "input.background": self.input.background,
                "input.foreground": self.input.text,
                "input.placeholderForeground": self.input.placeholder,
                "input.border": self.input.border,
                "errorForeground": self.state.danger
            },
            "tokenColors": []
        });
        Ok(serde_json::to_string_pretty(&theme)?)
    }

    fn export_windows_terminal_json(&self) -> Result<String, ThemeError> {
        let scheme = json!({
            "name": self.name,
            "background": self.terminal.background,
            "foreground": self.terminal.foreground,
            "cursorColor": self.terminal.cursor,
            "selectionBackground": self.terminal.selection,
            "black": self.terminal.ansi.black,
            "red": self.terminal.ansi.red,
            "green": self.terminal.ansi.green,
            "yellow": self.terminal.ansi.yellow,
            "blue": self.terminal.ansi.blue,
            "purple": self.terminal.ansi.magenta,
            "cyan": self.terminal.ansi.cyan,
            "white": self.terminal.ansi.white,
            "brightBlack": self.terminal.ansi.bright_black,
            "brightRed": self.terminal.ansi.bright_red,
            "brightGreen": self.terminal.ansi.bright_green,
            "brightYellow": self.terminal.ansi.bright_yellow,
            "brightBlue": self.terminal.ansi.bright_blue,
            "brightPurple": self.terminal.ansi.bright_magenta,
            "brightCyan": self.terminal.ansi.bright_cyan,
            "brightWhite": self.terminal.ansi.bright_white
        });
        Ok(serde_json::to_string_pretty(&scheme)?)
    }

    fn export_alacritty_toml(&self) -> String {
        format!(
            r#"[font]
normal = {{ family = "{}" }}
size = {}

[colors.primary]
background = "{}"
foreground = "{}"

[colors.cursor]
text = "{}"
cursor = "{}"

[colors.selection]
text = "{}"
background = "{}"

[colors.normal]
black = "{}"
red = "{}"
green = "{}"
yellow = "{}"
blue = "{}"
magenta = "{}"
cyan = "{}"
white = "{}"

[colors.bright]
black = "{}"
red = "{}"
green = "{}"
yellow = "{}"
blue = "{}"
magenta = "{}"
cyan = "{}"
white = "{}"
"#,
            self.font.terminal.family,
            self.font.terminal.size,
            self.terminal.background,
            self.terminal.foreground,
            self.terminal.selection_text,
            self.terminal.cursor,
            self.terminal.selection_text,
            self.terminal.selection,
            self.terminal.ansi.black,
            self.terminal.ansi.red,
            self.terminal.ansi.green,
            self.terminal.ansi.yellow,
            self.terminal.ansi.blue,
            self.terminal.ansi.magenta,
            self.terminal.ansi.cyan,
            self.terminal.ansi.white,
            self.terminal.ansi.bright_black,
            self.terminal.ansi.bright_red,
            self.terminal.ansi.bright_green,
            self.terminal.ansi.bright_yellow,
            self.terminal.ansi.bright_blue,
            self.terminal.ansi.bright_magenta,
            self.terminal.ansi.bright_cyan,
            self.terminal.ansi.bright_white,
        )
    }
}

pub fn built_in_theme_document(theme: BuiltInTheme) -> ThemeDocument {
    let (id, name, source, palette) = match theme {
        BuiltInTheme::ProfessionalDark => (
            "professional-dark",
            "Professional Dark",
            "builtin",
            palette(
                0x080d12, 0x0e131a, 0x090e14, 0x0d1219, 0x111721, 0x181f29, 0x05090d, 0x2f3a4a,
                0x202b38, 0x33405a, 0x35c88f, 0x4cc9f0, 0xc58cff, 0xf4b860, 0xf46060, 0xecf0f6,
                0xf3f6fb, 0x7f8b99, 0x91a0b2, 0xc6eed6, 0x5f7870,
            ),
        ),
        BuiltInTheme::CatppuccinMocha => (
            "catppuccin-mocha",
            "Catppuccin Mocha",
            "catppuccin",
            palette(
                0x1e1e2e, 0x181825, 0x11111b, 0x181825, 0x313244, 0x45475a, 0x11111b, 0x585b70,
                0x313244, 0x6c7086, 0xa6e3a1, 0x89b4fa, 0xcba6f7, 0xf9e2af, 0xf38ba8, 0xcdd6f4,
                0xf5e0dc, 0xa6adc8, 0xbac2de, 0xa6e3a1, 0x6c7086,
            ),
        ),
        BuiltInTheme::NordDark => (
            "nord-dark",
            "Nord Dark",
            "nord",
            palette(
                0x2e3440, 0x3b4252, 0x2e3440, 0x3b4252, 0x434c5e, 0x4c566a, 0x242933, 0x4c566a,
                0x3b4252, 0x5e81ac, 0xa3be8c, 0x88c0d0, 0xb48ead, 0xebcb8b, 0xbf616a, 0xeceff4,
                0xf8fbff, 0x9aa7b8, 0xd8dee9, 0xa3be8c, 0x81a1c1,
            ),
        ),
        BuiltInTheme::Dracula => (
            "dracula",
            "Dracula",
            "dracula",
            palette(
                0x282a36, 0x21222c, 0x191a21, 0x21222c, 0x282a36, 0x343746, 0x191a21, 0x44475a,
                0x343746, 0x6272a4, 0x50fa7b, 0x8be9fd, 0xbd93f9, 0xffb86c, 0xff5555, 0xf8f8f2,
                0xffffff, 0x9aa0c7, 0xb8c1ec, 0x50fa7b, 0x6272a4,
            ),
        ),
        BuiltInTheme::SolarizedDark => (
            "solarized-dark",
            "Solarized Dark",
            "solarized",
            palette(
                0x002b36, 0x073642, 0x00212a, 0x073642, 0x0b3a45, 0x164956, 0x001f27, 0x586e75,
                0x164956, 0x657b83, 0x859900, 0x268bd2, 0x6c71c4, 0xb58900, 0xdc322f, 0x839496,
                0x93a1a1, 0x657b83, 0x93a1a1, 0x2aa198, 0x586e75,
            ),
        ),
        BuiltInTheme::OceanDark => (
            "ocean-dark",
            "Ocean Dark",
            "builtin",
            palette(
                0x07131f, 0x0b1b2b, 0x06101a, 0x0a1724, 0x102235, 0x173047, 0x03101a, 0x2b4b64,
                0x173047, 0x3f6c8b, 0x45d3b1, 0x5bbcff, 0x9b8cff, 0xf4c56a, 0xff6675, 0xe8f4ff,
                0xf5fbff, 0x7f9ab0, 0xa8bfd2, 0x9debd8, 0x5f8794,
            ),
        ),
        BuiltInTheme::ForestDark => (
            "forest-dark",
            "Forest Dark",
            "builtin",
            palette(
                0x10160f, 0x151d14, 0x0b120b, 0x121a11, 0x182316, 0x22301f, 0x070d08, 0x354833,
                0x243323, 0x45613f, 0x8fd17d, 0x72c7a4, 0xd1a3ff, 0xe5c76b, 0xf07178, 0xf0f7ea,
                0xfbfff5, 0x87967f, 0xb0c0a8, 0xb7f3a4, 0x60795c,
            ),
        ),
    };

    ThemeDocument::from_palette(id, name, source, palette)
}

pub fn built_in_palette(theme: BuiltInTheme) -> ResolvedThemePalette {
    built_in_theme_document(theme)
        .resolve_palette()
        .expect("内置主题必须满足 schema")
}

impl ThemeDocument {
    fn from_palette(id: &str, name: &str, source: &str, palette: ResolvedThemePalette) -> Self {
        let color = |value| format_color(value);

        Self {
            schema_version: 1,
            id: id.to_owned(),
            name: name.to_owned(),
            kind: ThemeKind::Dark,
            extends: None,
            meta: ThemeMeta {
                source: source.to_owned(),
                ..ThemeMeta::default()
            },
            font: ThemeFonts::default(),
            frame: FrameColors {
                window: color(palette.window_bg),
                titlebar: color(palette.topbar_bg),
                topbar: color(palette.topbar_bg),
                rail: color(palette.rail_bg),
                left_sidebar: color(palette.panel_bg),
                right_sidebar: color(palette.topbar_bg),
                splitter: color(palette.border_soft),
                overlay: color(palette.overlay_bg),
            },
            workspace: WorkspaceColors {
                background: color(palette.inset_bg),
                panel: color(palette.surface_pressed),
                surface: color(palette.surface_bg),
                surface_hover: color(palette.surface_hover),
                surface_raised: color(palette.raised_bg),
                card: color(palette.card_bg),
                card_hover: color(palette.card_hover),
            },
            border: BorderColors {
                normal: color(palette.border),
                soft: color(palette.border_soft),
                strong: color(palette.border_strong),
                focus: color(palette.border_focus),
                danger: color(palette.border_danger),
            },
            text: TextColors {
                normal: color(palette.text),
                strong: color(palette.text_strong),
                muted: color(palette.text_muted),
                soft: color(palette.text_soft),
                disabled: color(palette.text_disabled),
                inverse: color(palette.text_inverse),
            },
            state: StateColors {
                success: color(palette.accent),
                info: color(palette.accent_blue),
                warning: color(palette.warning),
                danger: color(palette.danger),
                pending: color(palette.accent_violet),
            },
            input: InputColors {
                background: color(palette.input_bg),
                background_focus: color(palette.input_bg_focus),
                border: color(palette.border_soft),
                border_focus: color(palette.border_focus),
                text: color(palette.text),
                placeholder: color(palette.input_placeholder),
                selection: color(palette.input_selection),
                cursor: color(palette.text),
            },
            button: ButtonColors {
                primary: ButtonVariantColors {
                    background: color(palette.button_primary_bg),
                    background_hover: color(palette.button_primary_hover),
                    border: color(palette.button_primary_border),
                    text: color(palette.button_primary_text),
                },
                secondary: ButtonVariantColors {
                    background: color(palette.button_secondary_bg),
                    background_hover: color(palette.button_secondary_hover),
                    border: color(palette.button_secondary_border),
                    text: color(palette.button_secondary_text),
                },
            },
            dialog: DialogColors {
                background: color(palette.dialog_bg),
                header_icon_background: color(palette.dialog_danger_icon_bg),
                header_icon_border: color(palette.dialog_danger_icon_border),
                footer_border: color(palette.border_soft),
            },
            badge: BadgeColors {
                success_background: color(palette.badge_success_bg),
                info_background: color(palette.badge_info_bg),
                pending_background: color(palette.badge_pending_bg),
                warning_background: color(palette.badge_warning_bg),
            },
            tabs: TabColors {
                background: color(palette.tab_bg),
                active_background: color(palette.tab_active_bg),
                active_border: color(palette.tab_active_border),
                text: color(palette.text),
                muted: color(palette.text_muted),
            },
            terminal: TerminalColors {
                background: color(palette.terminal_bg),
                foreground: color(palette.terminal_text),
                muted: color(palette.terminal_muted),
                cursor: color(palette.terminal_text),
                selection: color(palette.terminal_selection),
                selection_text: color(palette.terminal_text),
                ansi: default_ansi(&palette),
            },
            background: BackgroundColors {
                color: color(palette.window_bg),
                ..BackgroundColors::default()
            },
        }
    }
}

fn default_ansi(palette: &ResolvedThemePalette) -> AnsiColors {
    AnsiColors {
        black: "#0b1016".to_owned(),
        red: format_color(palette.danger),
        green: format_color(palette.accent),
        yellow: format_color(palette.warning),
        blue: format_color(palette.accent_blue),
        magenta: format_color(palette.accent_violet),
        cyan: "#5eead4".to_owned(),
        white: format_color(palette.text),
        bright_black: format_color(palette.terminal_muted),
        bright_red: "#ff7a7a".to_owned(),
        bright_green: "#68e0aa".to_owned(),
        bright_yellow: "#ffd37a".to_owned(),
        bright_blue: "#78d8ff".to_owned(),
        bright_magenta: "#d8b4ff".to_owned(),
        bright_cyan: "#8ff7e6".to_owned(),
        bright_white: "#ffffff".to_owned(),
    }
}

fn palette(
    window_bg: u32,
    topbar_bg: u32,
    rail_bg: u32,
    panel_bg: u32,
    surface_bg: u32,
    raised_bg: u32,
    terminal_bg: u32,
    border: u32,
    border_soft: u32,
    border_strong: u32,
    accent: u32,
    accent_blue: u32,
    accent_violet: u32,
    warning: u32,
    danger: u32,
    text: u32,
    text_strong: u32,
    text_muted: u32,
    text_soft: u32,
    terminal_text: u32,
    terminal_muted: u32,
) -> ResolvedThemePalette {
    let window_bg = rgb(window_bg);
    let topbar_bg = rgb(topbar_bg);
    let rail_bg = rgb(rail_bg);
    let panel_bg = rgb(panel_bg);
    let surface_bg = rgb(surface_bg);
    let raised_bg = rgb(raised_bg);
    let terminal_bg = rgb(terminal_bg);
    let border = rgb(border);
    let border_soft = rgb(border_soft);
    let border_strong = rgb(border_strong);
    let accent = rgb(accent);
    let accent_blue = rgb(accent_blue);
    let accent_violet = rgb(accent_violet);
    let warning = rgb(warning);
    let danger = rgb(danger);
    let text = rgb(text);
    let text_strong = rgb(text_strong);
    let text_muted = rgb(text_muted);
    let text_soft = rgb(text_soft);
    let terminal_text = rgb(terminal_text);
    let terminal_muted = rgb(terminal_muted);

    ResolvedThemePalette {
        window_bg,
        topbar_bg,
        rail_bg,
        panel_bg,
        surface_bg,
        surface_hover: raised_bg,
        surface_pressed: panel_bg,
        raised_bg,
        card_bg: surface_bg,
        card_hover: raised_bg,
        inset_bg: window_bg,
        dock_bg: 0xe610_1721,
        terminal_bg,
        overlay_bg: 0x99000000,
        border,
        border_soft,
        border_strong,
        border_focus: accent_blue,
        border_danger: danger,
        accent,
        accent_blue,
        accent_violet,
        warning,
        danger,
        text,
        text_strong,
        text_muted,
        text_soft,
        text_secondary: text_soft,
        text_disabled: text_muted,
        text_inverse: rgb(0xffffff),
        section_text: text_muted,
        status_muted: text_soft,
        success_text: accent,
        success_text_soft: terminal_text,
        info_text: accent_blue,
        danger_text: danger,
        danger_text_soft: rgb(0xff7a7a),
        badge_success_bg: surface_bg,
        badge_info_bg: raised_bg,
        badge_pending_bg: raised_bg,
        badge_warning_bg: surface_bg,
        selection_bg: raised_bg,
        input_bg: surface_bg,
        input_bg_focus: raised_bg,
        input_placeholder: text_muted,
        input_selection: rgb(0x2c7068),
        button_primary_bg: raised_bg,
        button_primary_hover: accent_blue,
        button_primary_pressed: panel_bg,
        button_primary_border: border,
        button_primary_border_hover: border_focus_fallback(accent_blue),
        button_primary_border_pressed: border_strong,
        button_primary_text: text_strong,
        button_secondary_bg: surface_bg,
        button_secondary_hover: raised_bg,
        button_secondary_pressed: panel_bg,
        button_secondary_border: border_soft,
        button_secondary_text: text,
        button_subtle_hover: raised_bg,
        button_subtle_pressed: surface_bg,
        button_danger_bg: surface_bg,
        button_danger_hover: raised_bg,
        button_danger_pressed: danger,
        button_danger_border: danger,
        button_danger_border_hover: danger,
        button_danger_border_pressed: danger,
        button_danger_text: danger,
        dialog_bg: panel_bg,
        dialog_section_bg: surface_bg,
        dialog_success_icon_bg: surface_bg,
        dialog_success_icon_border: accent,
        dialog_danger_icon_bg: surface_bg,
        dialog_danger_icon_border: danger,
        dialog_error_bg: surface_bg,
        dialog_error_border: danger,
        tab_bg: surface_bg,
        tab_hover_bg: raised_bg,
        tab_pressed_bg: panel_bg,
        tab_active_bg: raised_bg,
        tab_active_border: accent_blue,
        tab_accent_hover: raised_bg,
        tab_close_hover_bg: raised_bg,
        tab_close_pressed_bg: surface_bg,
        tab_close_border_hover: border_strong,
        terminal_viewport_bg: terminal_bg,
        terminal_border: border_soft,
        terminal_selection: rgb(0x2c7068),
        topbar_border: border_soft,
        terminal_text,
        terminal_muted,
    }
}

const fn border_focus_fallback(value: u32) -> u32 {
    value
}

fn theme_document_from_json(input: &str, source_name: &str) -> Result<ThemeDocument, ThemeError> {
    let value: serde_json::Value = serde_json::from_str(input).map_err(ThemeError::InvalidJson)?;
    if let Some(scheme) = windows_terminal_scheme(&value) {
        return theme_document_from_windows_terminal_json(scheme, source_name);
    }
    if value.get("colors").is_some() {
        return theme_document_from_vscode_json(&value, source_name);
    }

    Err(ThemeError::InvalidExternalTheme("无法识别 JSON 主题格式"))
}

fn theme_document_from_vscode_json(
    value: &serde_json::Value,
    source_name: &str,
) -> Result<ThemeDocument, ThemeError> {
    let colors = value
        .get("colors")
        .and_then(serde_json::Value::as_object)
        .ok_or(ThemeError::InvalidExternalTheme("VS Code 主题缺少 colors"))?;
    let name = json_string(value, "name")
        .map(str::to_owned)
        .unwrap_or_else(|| imported_theme_name(source_name, "VS Code Theme"));
    let mut document = external_theme_base(&name, source_name, "vscode");
    document.kind = match json_string(value, "type") {
        Some("light") => ThemeKind::Light,
        Some("hc") | Some("highContrast") => ThemeKind::HighContrast,
        _ => ThemeKind::Dark,
    };

    document.terminal.background = required_json_color(
        colors,
        &["terminal.background", "editor.background"],
        "terminal.background",
    )?;
    document.terminal.foreground = required_json_color(
        colors,
        &["terminal.foreground", "editor.foreground"],
        "terminal.foreground",
    )?;
    assign_json_color(
        colors,
        &["terminal.selectionBackground", "editor.selectionBackground"],
        "terminal.selection",
        &mut document.terminal.selection,
    )?;
    assign_json_color(
        colors,
        &["terminal.foreground", "editor.foreground"],
        "terminal.selection_text",
        &mut document.terminal.selection_text,
    )?;
    assign_json_color(
        colors,
        &["terminal.foreground", "editor.foreground"],
        "terminal.cursor",
        &mut document.terminal.cursor,
    )?;
    assign_json_color(
        colors,
        &["foreground"],
        "text.normal",
        &mut document.text.normal,
    )?;
    assign_json_color(
        colors,
        &["focusBorder"],
        "border.focus",
        &mut document.border.focus,
    )?;
    assign_json_color(
        colors,
        &["titleBar.activeBackground"],
        "frame.titlebar",
        &mut document.frame.titlebar,
    )?;
    assign_json_color(
        colors,
        &["activityBar.background"],
        "frame.rail",
        &mut document.frame.rail,
    )?;
    assign_json_color(
        colors,
        &["sideBar.background"],
        "frame.left_sidebar",
        &mut document.frame.left_sidebar,
    )?;
    assign_json_color(
        colors,
        &["panel.background"],
        "workspace.panel",
        &mut document.workspace.panel,
    )?;
    assign_vscode_ansi(colors, &mut document.terminal.ansi)?;
    document.validate()?;
    Ok(document)
}

fn theme_document_from_windows_terminal_json(
    value: &serde_json::Value,
    source_name: &str,
) -> Result<ThemeDocument, ThemeError> {
    let name = json_string(value, "name")
        .map(str::to_owned)
        .unwrap_or_else(|| imported_theme_name(source_name, "Windows Terminal Theme"));
    let mut document = external_theme_base(&name, source_name, "windows-terminal");

    document.terminal.background = required_value_color(value, "background")?;
    document.terminal.foreground = required_value_color(value, "foreground")?;
    assign_value_color(
        value,
        "cursorColor",
        "cursorColor",
        &mut document.terminal.cursor,
    )?;
    assign_value_color(
        value,
        "selectionBackground",
        "selectionBackground",
        &mut document.terminal.selection,
    )?;
    assign_value_color(value, "black", "black", &mut document.terminal.ansi.black)?;
    assign_value_color(value, "red", "red", &mut document.terminal.ansi.red)?;
    assign_value_color(value, "green", "green", &mut document.terminal.ansi.green)?;
    assign_value_color(
        value,
        "yellow",
        "yellow",
        &mut document.terminal.ansi.yellow,
    )?;
    assign_value_color(value, "blue", "blue", &mut document.terminal.ansi.blue)?;
    assign_value_color(
        value,
        "purple",
        "purple",
        &mut document.terminal.ansi.magenta,
    )?;
    assign_value_color(value, "cyan", "cyan", &mut document.terminal.ansi.cyan)?;
    assign_value_color(value, "white", "white", &mut document.terminal.ansi.white)?;
    assign_value_color(
        value,
        "brightBlack",
        "brightBlack",
        &mut document.terminal.ansi.bright_black,
    )?;
    assign_value_color(
        value,
        "brightRed",
        "brightRed",
        &mut document.terminal.ansi.bright_red,
    )?;
    assign_value_color(
        value,
        "brightGreen",
        "brightGreen",
        &mut document.terminal.ansi.bright_green,
    )?;
    assign_value_color(
        value,
        "brightYellow",
        "brightYellow",
        &mut document.terminal.ansi.bright_yellow,
    )?;
    assign_value_color(
        value,
        "brightBlue",
        "brightBlue",
        &mut document.terminal.ansi.bright_blue,
    )?;
    assign_value_color(
        value,
        "brightPurple",
        "brightPurple",
        &mut document.terminal.ansi.bright_magenta,
    )?;
    assign_value_color(
        value,
        "brightCyan",
        "brightCyan",
        &mut document.terminal.ansi.bright_cyan,
    )?;
    assign_value_color(
        value,
        "brightWhite",
        "brightWhite",
        &mut document.terminal.ansi.bright_white,
    )?;
    document.validate()?;
    Ok(document)
}

fn theme_document_from_alacritty_toml(
    input: &str,
    source_name: &str,
) -> Result<ThemeDocument, ThemeError> {
    let value: toml::Value = toml::from_str(input)?;
    let name = imported_theme_name(source_name, "Alacritty Theme");
    let mut document = external_theme_base(&name, source_name, "alacritty");

    document.terminal.background = required_toml_color(
        &value,
        &["colors", "primary", "background"],
        "colors.primary.background",
    )?;
    document.terminal.foreground = required_toml_color(
        &value,
        &["colors", "primary", "foreground"],
        "colors.primary.foreground",
    )?;
    assign_toml_color(
        &value,
        &["colors", "cursor", "cursor"],
        "colors.cursor.cursor",
        &mut document.terminal.cursor,
    )?;
    assign_toml_color(
        &value,
        &["colors", "selection", "background"],
        "colors.selection.background",
        &mut document.terminal.selection,
    )?;
    assign_toml_color(
        &value,
        &["colors", "selection", "text"],
        "colors.selection.text",
        &mut document.terminal.selection_text,
    )?;
    assign_toml_ansi(&value, &mut document.terminal.ansi)?;
    if let Some(family) = toml_path_str(&value, &["font", "normal", "family"]) {
        document.font.terminal.family = family.to_owned();
    }
    if let Some(size) = toml_path_f64(&value, &["font", "size"]) {
        document.font.terminal.size = size.round().max(1.0) as u16;
    }
    document.validate()?;
    Ok(document)
}

fn windows_terminal_scheme(value: &serde_json::Value) -> Option<&serde_json::Value> {
    if value.get("background").is_some() && value.get("foreground").is_some() {
        return Some(value);
    }
    value
        .get("schemes")
        .and_then(serde_json::Value::as_array)
        .and_then(|schemes| {
            schemes.iter().find(|scheme| {
                scheme.get("background").is_some() && scheme.get("foreground").is_some()
            })
        })
}

fn external_theme_base(name: &str, source_name: &str, source_kind: &str) -> ThemeDocument {
    let mut document = built_in_theme_document(BuiltInTheme::ProfessionalDark);
    document.id = slugify_theme_id(name);
    document.name = name.to_owned();
    document.meta.source = format!("{source_kind}:{source_name}");
    document
}

fn imported_theme_name(source_name: &str, fallback: &str) -> String {
    Path::new(source_name)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::trim)
        .filter(|stem| !stem.is_empty())
        .unwrap_or(fallback)
        .to_owned()
}

fn import_extension(source_name: &str) -> Option<&'static str> {
    let extension = Path::new(source_name).extension()?.to_str()?;
    if extension.eq_ignore_ascii_case("json") {
        Some("json")
    } else if extension.eq_ignore_ascii_case("toml") {
        Some("toml")
    } else if extension.eq_ignore_ascii_case("itermcolors") {
        Some("itermcolors")
    } else {
        None
    }
}

fn looks_like_json(input: &str) -> bool {
    input.trim_start().starts_with('{') || input.trim_start().starts_with('[')
}

fn looks_like_alacritty_toml(input: &str) -> bool {
    input.contains("[colors.primary]") || input.contains("[colors.normal]")
}

fn json_string<'a>(value: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(serde_json::Value::as_str)
}

fn required_json_color(
    colors: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
    field: &'static str,
) -> Result<String, ThemeError> {
    keys.iter()
        .find_map(|key| colors.get(*key).and_then(serde_json::Value::as_str))
        .ok_or(ThemeError::InvalidExternalTheme(field))
        .and_then(|value| normalize_color(field, value))
}

fn assign_json_color(
    colors: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
    field: &'static str,
    target: &mut String,
) -> Result<(), ThemeError> {
    if let Some(value) = keys
        .iter()
        .find_map(|key| colors.get(*key).and_then(serde_json::Value::as_str))
    {
        *target = normalize_color(field, value)?;
    }
    Ok(())
}

fn required_value_color(
    value: &serde_json::Value,
    field: &'static str,
) -> Result<String, ThemeError> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or(ThemeError::InvalidExternalTheme(field))
        .and_then(|color| normalize_color(field, color))
}

fn assign_value_color(
    value: &serde_json::Value,
    key: &str,
    field: &'static str,
    target: &mut String,
) -> Result<(), ThemeError> {
    if let Some(color) = value.get(key).and_then(serde_json::Value::as_str) {
        *target = normalize_color(field, color)?;
    }
    Ok(())
}

fn required_toml_color(
    value: &toml::Value,
    path: &[&str],
    field: &'static str,
) -> Result<String, ThemeError> {
    toml_path_str(value, path)
        .ok_or(ThemeError::InvalidExternalTheme(field))
        .and_then(|color| normalize_color(field, color))
}

fn assign_toml_color(
    value: &toml::Value,
    path: &[&str],
    field: &'static str,
    target: &mut String,
) -> Result<(), ThemeError> {
    if let Some(color) = toml_path_str(value, path) {
        *target = normalize_color(field, color)?;
    }
    Ok(())
}

fn assign_vscode_ansi(
    colors: &serde_json::Map<String, serde_json::Value>,
    ansi: &mut AnsiColors,
) -> Result<(), ThemeError> {
    assign_json_color(
        colors,
        &["terminal.ansiBlack"],
        "terminal.ansiBlack",
        &mut ansi.black,
    )?;
    assign_json_color(
        colors,
        &["terminal.ansiRed"],
        "terminal.ansiRed",
        &mut ansi.red,
    )?;
    assign_json_color(
        colors,
        &["terminal.ansiGreen"],
        "terminal.ansiGreen",
        &mut ansi.green,
    )?;
    assign_json_color(
        colors,
        &["terminal.ansiYellow"],
        "terminal.ansiYellow",
        &mut ansi.yellow,
    )?;
    assign_json_color(
        colors,
        &["terminal.ansiBlue"],
        "terminal.ansiBlue",
        &mut ansi.blue,
    )?;
    assign_json_color(
        colors,
        &["terminal.ansiMagenta"],
        "terminal.ansiMagenta",
        &mut ansi.magenta,
    )?;
    assign_json_color(
        colors,
        &["terminal.ansiCyan"],
        "terminal.ansiCyan",
        &mut ansi.cyan,
    )?;
    assign_json_color(
        colors,
        &["terminal.ansiWhite"],
        "terminal.ansiWhite",
        &mut ansi.white,
    )?;
    assign_json_color(
        colors,
        &["terminal.ansiBrightBlack"],
        "terminal.ansiBrightBlack",
        &mut ansi.bright_black,
    )?;
    assign_json_color(
        colors,
        &["terminal.ansiBrightRed"],
        "terminal.ansiBrightRed",
        &mut ansi.bright_red,
    )?;
    assign_json_color(
        colors,
        &["terminal.ansiBrightGreen"],
        "terminal.ansiBrightGreen",
        &mut ansi.bright_green,
    )?;
    assign_json_color(
        colors,
        &["terminal.ansiBrightYellow"],
        "terminal.ansiBrightYellow",
        &mut ansi.bright_yellow,
    )?;
    assign_json_color(
        colors,
        &["terminal.ansiBrightBlue"],
        "terminal.ansiBrightBlue",
        &mut ansi.bright_blue,
    )?;
    assign_json_color(
        colors,
        &["terminal.ansiBrightMagenta"],
        "terminal.ansiBrightMagenta",
        &mut ansi.bright_magenta,
    )?;
    assign_json_color(
        colors,
        &["terminal.ansiBrightCyan"],
        "terminal.ansiBrightCyan",
        &mut ansi.bright_cyan,
    )?;
    assign_json_color(
        colors,
        &["terminal.ansiBrightWhite"],
        "terminal.ansiBrightWhite",
        &mut ansi.bright_white,
    )?;
    Ok(())
}

fn assign_toml_ansi(value: &toml::Value, ansi: &mut AnsiColors) -> Result<(), ThemeError> {
    assign_toml_color(
        value,
        &["colors", "normal", "black"],
        "colors.normal.black",
        &mut ansi.black,
    )?;
    assign_toml_color(
        value,
        &["colors", "normal", "red"],
        "colors.normal.red",
        &mut ansi.red,
    )?;
    assign_toml_color(
        value,
        &["colors", "normal", "green"],
        "colors.normal.green",
        &mut ansi.green,
    )?;
    assign_toml_color(
        value,
        &["colors", "normal", "yellow"],
        "colors.normal.yellow",
        &mut ansi.yellow,
    )?;
    assign_toml_color(
        value,
        &["colors", "normal", "blue"],
        "colors.normal.blue",
        &mut ansi.blue,
    )?;
    assign_toml_color(
        value,
        &["colors", "normal", "magenta"],
        "colors.normal.magenta",
        &mut ansi.magenta,
    )?;
    assign_toml_color(
        value,
        &["colors", "normal", "cyan"],
        "colors.normal.cyan",
        &mut ansi.cyan,
    )?;
    assign_toml_color(
        value,
        &["colors", "normal", "white"],
        "colors.normal.white",
        &mut ansi.white,
    )?;
    assign_toml_color(
        value,
        &["colors", "bright", "black"],
        "colors.bright.black",
        &mut ansi.bright_black,
    )?;
    assign_toml_color(
        value,
        &["colors", "bright", "red"],
        "colors.bright.red",
        &mut ansi.bright_red,
    )?;
    assign_toml_color(
        value,
        &["colors", "bright", "green"],
        "colors.bright.green",
        &mut ansi.bright_green,
    )?;
    assign_toml_color(
        value,
        &["colors", "bright", "yellow"],
        "colors.bright.yellow",
        &mut ansi.bright_yellow,
    )?;
    assign_toml_color(
        value,
        &["colors", "bright", "blue"],
        "colors.bright.blue",
        &mut ansi.bright_blue,
    )?;
    assign_toml_color(
        value,
        &["colors", "bright", "magenta"],
        "colors.bright.magenta",
        &mut ansi.bright_magenta,
    )?;
    assign_toml_color(
        value,
        &["colors", "bright", "cyan"],
        "colors.bright.cyan",
        &mut ansi.bright_cyan,
    )?;
    assign_toml_color(
        value,
        &["colors", "bright", "white"],
        "colors.bright.white",
        &mut ansi.bright_white,
    )?;
    Ok(())
}

fn toml_path_str<'a>(value: &'a toml::Value, path: &[&str]) -> Option<&'a str> {
    path.iter()
        .try_fold(value, |current, part| current.get(*part))
        .and_then(toml::Value::as_str)
}

fn toml_path_f64(value: &toml::Value, path: &[&str]) -> Option<f64> {
    let value = path
        .iter()
        .try_fold(value, |current, part| current.get(*part))?;
    value
        .as_float()
        .or_else(|| value.as_integer().map(|number| number as f64))
}

fn normalize_color(field: &'static str, value: &str) -> Result<String, ThemeError> {
    parse_color(field, value).map(format_color)
}

fn slugify_theme_id(name: &str) -> String {
    let mut id = String::new();
    let mut last_was_dash = false;
    for ch in name.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            id.push(ch);
            last_was_dash = false;
        } else if !last_was_dash && !id.is_empty() {
            id.push('-');
            last_was_dash = true;
        }
    }
    let trimmed = id.trim_matches('-').to_owned();
    if trimmed.is_empty() {
        "imported-theme".to_owned()
    } else {
        trimmed
    }
}

fn parse_color(field: &'static str, value: &str) -> Result<u32, ThemeError> {
    let raw = value.trim().strip_prefix('#').unwrap_or(value.trim());
    let parsed = match raw.len() {
        6 => u32::from_str_radix(raw, 16).map(|rgb| 0xff000000 | rgb),
        8 => u32::from_str_radix(raw, 16),
        _ => {
            return Err(ThemeError::InvalidColor {
                field,
                value: value.to_owned(),
            });
        }
    };

    parsed.map_err(|_| ThemeError::InvalidColor {
        field,
        value: value.to_owned(),
    })
}

fn format_color(argb: u32) -> String {
    let alpha = (argb >> 24) & 0xff;
    if alpha == 0xff {
        format!("#{:06x}", argb & 0x00ff_ffff)
    } else {
        format!("#{:08x}", argb)
    }
}

const fn rgb(value: u32) -> u32 {
    0xff000000 | value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_themes_resolve_to_palette() {
        let palette = built_in_palette(BuiltInTheme::CatppuccinMocha);

        assert_eq!(palette.window_bg, 0xff1e1e2e);
        assert_eq!(palette.accent, 0xffa6e3a1);
    }

    #[test]
    fn native_theme_round_trips_through_toml() {
        let document = built_in_theme_document(BuiltInTheme::NordDark);
        let encoded = document.to_toml().expect("内置主题应该可以导出");
        let decoded = ThemeDocument::from_toml(&encoded).expect("导出的主题应该可以重新导入");

        assert!(encoded.contains("[badge]"));
        assert_eq!(decoded.id, "nord-dark");
        assert_eq!(
            decoded.resolve_palette().expect("导入主题应该可解析"),
            document.resolve_palette().expect("原主题应该可解析")
        );
    }

    #[test]
    fn exports_common_theme_formats_with_reports() {
        let document = built_in_theme_document(BuiltInTheme::Dracula);

        let native = document
            .export(ThemeExchangeFormat::NativeToml)
            .expect("原生主题应该可以导出");
        assert!(!native.report.lossy);
        assert!(native.content.contains("[terminal.ansi]"));

        let vscode = document
            .export(ThemeExchangeFormat::VsCodeJson)
            .expect("VSCode 主题应该可以导出");
        assert!(vscode.report.lossy);
        assert!(vscode.content.contains("terminal.ansiRed"));

        let windows_terminal = document
            .export(ThemeExchangeFormat::WindowsTerminalJson)
            .expect("Windows Terminal 主题应该可以导出");
        assert!(!windows_terminal.report.lossy);
        assert!(windows_terminal.content.contains("brightPurple"));
    }

    #[test]
    fn iterm2_export_reports_explicit_gap() {
        let document = built_in_theme_document(BuiltInTheme::Dracula);

        assert!(matches!(
            document.export(ThemeExchangeFormat::ItermColors),
            Err(ThemeError::UnsupportedFormat(_))
        ));
    }

    #[test]
    fn imports_common_external_theme_formats() {
        let mut source = built_in_theme_document(BuiltInTheme::Dracula);
        source.name = "External Dracula".to_owned();

        let vscode = source
            .export(ThemeExchangeFormat::VsCodeJson)
            .expect("VS Code theme should export");
        let vscode_imported = ThemeDocument::from_import(&vscode.content, "dracula.json")
            .expect("VS Code theme should import");
        assert_eq!(vscode_imported.name, "External Dracula");
        assert_eq!(vscode_imported.terminal.ansi.red, source.terminal.ansi.red);

        let windows_terminal = source
            .export(ThemeExchangeFormat::WindowsTerminalJson)
            .expect("Windows Terminal theme should export");
        let windows_imported =
            ThemeDocument::from_import(&windows_terminal.content, "windows-terminal.json")
                .expect("Windows Terminal theme should import");
        assert_eq!(windows_imported.name, "External Dracula");
        assert_eq!(
            windows_imported.terminal.ansi.magenta,
            source.terminal.ansi.magenta
        );

        let alacritty = source
            .export(ThemeExchangeFormat::AlacrittyToml)
            .expect("Alacritty theme should export");
        let alacritty_imported = ThemeDocument::from_import(&alacritty.content, "dracula.toml")
            .expect("Alacritty theme should import");
        assert_eq!(alacritty_imported.name, "dracula");
        assert_eq!(
            alacritty_imported.terminal.background,
            source.terminal.background
        );
    }

    #[test]
    fn iterm2_import_reports_explicit_gap() {
        assert!(matches!(
            ThemeDocument::from_import("<plist></plist>", "theme.itermcolors"),
            Err(ThemeError::UnsupportedFormat(_))
        ));
    }

    #[test]
    fn rejects_invalid_color_with_field_name() {
        let mut document = built_in_theme_document(BuiltInTheme::ProfessionalDark);
        document.frame.window = "not-a-color".to_owned();

        assert!(matches!(
            document.validate(),
            Err(ThemeError::InvalidColor {
                field: "frame.window",
                ..
            })
        ));
    }

    #[test]
    fn resolves_badge_warning_color_from_theme_document() {
        let mut document = built_in_theme_document(BuiltInTheme::ProfessionalDark);
        document.badge.warning_background = "#332211".to_owned();

        let palette = document.resolve_palette().expect("主题应该可以解析");

        assert_eq!(palette.badge_warning_bg, 0xff332211);
    }
}
