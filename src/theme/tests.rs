use super::*;
use crate::model::BuiltInTheme;

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
fn partial_native_theme_extends_builtin_and_overrides_selected_fields() {
    let input = r##"
schema_version = 1
id = "hand-tuned"
name = "Hand Tuned"
kind = "dark"
extends = "nord-dark"

[frame]
window = "#101418"

[text]
normal = "#dce7f2"

[overrides]
"button.primary.background_hover" = "#223344"
"terminal.selection" = "#334455"
"##;

    let document = ThemeDocument::from_toml(input).expect("精简主题应该可以导入");
    let palette = document.resolve_palette().expect("精简主题应该可以解析");

    assert_eq!(document.id, "hand-tuned");
    assert_eq!(document.name, "Hand Tuned");
    assert_eq!(document.frame.window, "#101418");
    assert_eq!(document.frame.topbar, "#3b4252");
    assert_eq!(document.text.normal, "#dce7f2");
    assert_eq!(document.button.primary.background_hover, "#223344");
    assert_eq!(document.terminal.selection, "#334455");
    assert_eq!(palette.window_bg, 0xff101418);
    assert_eq!(palette.button_primary_hover, 0xff223344);
    assert_eq!(palette.terminal_selection, 0xff334455);
}

#[test]
fn partial_native_theme_requires_identity() {
    let input = r##"
[frame]
window = "#101418"
"##;

    assert!(matches!(
        ThemeDocument::from_toml(input),
        Err(ThemeError::InvalidExternalTheme("id"))
    ));
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
