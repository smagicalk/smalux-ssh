//! 主题导入导出适配。

use std::path::Path;

use serde_json::json;

use crate::model::BuiltInTheme;

use super::builtin::built_in_theme_document;
use super::color::normalize_color;
use super::partial::theme_document_from_partial_toml;
use super::{
    AnsiColors, ExportedTheme, ThemeDocument, ThemeError, ThemeExchangeFormat, ThemeExchangeReport,
    ThemeKind,
};

impl ThemeDocument {
    /// 从 SmagicalSSH 原生 TOML 读取主题。
    pub fn from_toml(input: &str) -> Result<Self, ThemeError> {
        match toml::from_str::<Self>(input) {
            Ok(document) => {
                document.validate()?;
                Ok(document)
            }
            Err(full_error) => match theme_document_from_partial_toml(input) {
                Ok(document) => Ok(document),
                Err(ThemeError::InvalidToml(_)) => Err(ThemeError::InvalidToml(full_error)),
                Err(error) => Err(error),
            },
        }
    }

    /// 按文件名和内容自动识别并导入主题。
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

    /// 导出 SmagicalSSH 原生 TOML。
    pub fn to_toml(&self) -> Result<String, ThemeError> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// 导出为指定外部格式。
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
