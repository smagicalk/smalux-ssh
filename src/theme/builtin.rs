//! 内置主题和默认色板。

use crate::model::BuiltInTheme;

use super::color::{format_color, rgb};
use super::{
    AnsiColors, BackgroundColors, BadgeColors, BorderColors, ButtonColors, ButtonVariantColors,
    DialogColors, FrameColors, InputColors, ResolvedThemePalette, StateColors, TabColors,
    TerminalColors, TextColors, ThemeDocument, ThemeFonts, ThemeKind, ThemeMeta, WorkspaceColors,
};

/// 返回内置主题的原生主题文档。
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

    theme_document_from_palette(id, name, source, palette)
}

/// 返回内置主题解析后的 UI 色板。
pub fn built_in_palette(theme: BuiltInTheme) -> ResolvedThemePalette {
    built_in_theme_document(theme)
        .resolve_palette()
        .expect("内置主题必须满足 schema")
}

fn theme_document_from_palette(
    id: &str,
    name: &str,
    source: &str,
    palette: ResolvedThemePalette,
) -> ThemeDocument {
    let color = |value| format_color(value);

    ThemeDocument {
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
