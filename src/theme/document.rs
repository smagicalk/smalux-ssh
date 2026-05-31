//! 主题文件格式和对外交换类型。

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Slint UI 最终消费的扁平颜色表。
///
/// `ThemeDocument` 是面向配置文件和导入导出的结构化格式；这里的
/// `ResolvedThemePalette` 是面向渲染层的稳定投影，字段尽量和
/// `ui/theme.slint` 的全局 token 一一对应。
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

/// SmagicalSSH 原生主题文件。
///
/// 该结构是主题系统的中心契约：导入 VS Code、Windows Terminal、
/// Alacritty 等外部格式后，都会先转换成这个结构，再由解析层生成
/// `ResolvedThemePalette`。
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

/// 主题亮暗类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeKind {
    Dark,
    Light,
    HighContrast,
}

/// 主题来源和许可证等元信息。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ThemeMeta {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub author: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub license: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
}

/// UI 和终端字体配置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ThemeFonts {
    #[serde(default)]
    pub ui: FontSpec,
    #[serde(default)]
    pub terminal: TerminalFontSpec,
}

/// 普通 UI 字体。
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

/// 终端字体，额外包含行高。
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

/// 顶层窗口、标题栏、侧栏等框架颜色。
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

/// 工作区背景、卡片和浮起 surface 颜色。
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

/// 全局边框颜色。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BorderColors {
    pub normal: String,
    pub soft: String,
    pub strong: String,
    pub focus: String,
    pub danger: String,
}

/// 文本层级颜色。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextColors {
    pub normal: String,
    pub strong: String,
    pub muted: String,
    pub soft: String,
    pub disabled: String,
    pub inverse: String,
}

/// 成功、信息、警告、危险、待处理状态色。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateColors {
    pub success: String,
    pub info: String,
    pub warning: String,
    pub danger: String,
    pub pending: String,
}

/// 输入框颜色。
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

/// 按钮颜色配置。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ButtonColors {
    pub primary: ButtonVariantColors,
    pub secondary: ButtonVariantColors,
}

/// 单个按钮变体颜色。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ButtonVariantColors {
    pub background: String,
    pub background_hover: String,
    pub border: String,
    pub text: String,
}

/// 弹窗颜色。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DialogColors {
    pub background: String,
    pub header_icon_background: String,
    pub header_icon_border: String,
    pub footer_border: String,
}

/// 徽标背景色。
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

/// 终端 tab 颜色。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabColors {
    pub background: String,
    pub active_background: String,
    pub active_border: String,
    pub text: String,
    pub muted: String,
}

/// 终端基础颜色。
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

/// 标准 16 色 ANSI 调色板。
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

/// 背景图和背景色配置。
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

/// 主题导入、导出、解析过程中的统一错误类型。
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

/// 主题导入导出支持的外部格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeExchangeFormat {
    NativeToml,
    VsCodeJson,
    WindowsTerminalJson,
    AlacrittyToml,
    ItermColors,
}

/// 导出后的文件内容和转换报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportedTheme {
    pub file_name: String,
    pub content: String,
    pub report: ThemeExchangeReport,
}

/// 外部格式转换报告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeExchangeReport {
    pub format: ThemeExchangeFormat,
    pub lossy: bool,
    pub warnings: Vec<String>,
}
