//! 主界面视觉样式。

use iced::{
    Background, Border, Color, Shadow, Theme, Vector,
    widget::{button, container, text_input},
};

use crate::model::{AppState, BuiltInTheme};

pub(super) const TEXT_STRONG: Color = Color::from_rgb8(236, 240, 246);
pub(super) const TEXT_SOFT: Color = Color::from_rgb8(199, 208, 219);
pub(super) const TEXT_MUTED: Color = Color::from_rgb8(127, 139, 153);
pub(super) const TEXT_SUBTLE: Color = Color::from_rgb8(91, 103, 118);
pub(super) const ACCENT: Color = Color::from_rgb8(47, 201, 146);
pub(super) const BLUE: Color = Color::from_rgb8(94, 151, 246);
pub(super) const SURFACE: Color = Color::from_rgb8(18, 23, 31);
pub(super) const SURFACE_2: Color = Color::from_rgb8(24, 31, 41);
pub(super) const BORDER: Color = Color::from_rgb8(47, 58, 74);
pub(super) const TERMINAL_BG: Color = Color::from_rgb8(6, 10, 14);
pub(super) const TERMINAL_TEXT: Color = Color::from_rgb8(198, 238, 214);
pub(super) const TERMINAL_DIM: Color = Color::from_rgb8(95, 120, 112);

pub(super) fn app_background_style_for(state: &AppState) -> container::Style {
    let background = match state.ui.workspace.theme {
        BuiltInTheme::ProfessionalDark => Color::from_rgb8(11, 15, 21),
        BuiltInTheme::OceanDark => Color::from_rgb8(7, 22, 31),
        BuiltInTheme::ForestDark => Color::from_rgb8(9, 24, 19),
    };

    container::Style::default()
        .background(background)
        .color(TEXT_SOFT)
}

pub(super) fn title_bar_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(14, 19, 26))
        .border(Border::default().width(1).color(BORDER))
}

pub(super) fn rail_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(13, 18, 25))
        .border(Border::default().width(1).color(BORDER))
}

pub(super) fn nav_rail_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(10, 14, 20))
        .border(
            Border::default()
                .width(1)
                .color(Color::from_rgb8(35, 45, 58)),
        )
}

pub(super) fn workspace_style(_: &Theme) -> container::Style {
    container::Style::default().background(Color::from_rgb8(11, 15, 21))
}

pub(super) fn activity_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(14, 19, 26))
        .border(Border::default().width(1).color(BORDER))
}

pub(super) fn quick_connect_style(_: &Theme) -> container::Style {
    elevated_style(SURFACE_2, 8)
}

pub(super) fn host_card_style(_: &Theme) -> container::Style {
    elevated_style(SURFACE, 8).border(Border::default().rounded(8).width(1).color(BORDER))
}

pub(super) fn side_panel_style(_: &Theme) -> container::Style {
    elevated_style(SURFACE_2, 8)
}

pub(super) fn command_palette_style(_: &Theme) -> container::Style {
    elevated_style(Color::from_rgb8(17, 23, 32), 8)
        .border(Border::default().rounded(8).width(1).color(BLUE))
}

pub(super) fn terminal_style(_: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TERMINAL_TEXT),
        background: Some(Background::Color(TERMINAL_BG)),
        border: Border::default()
            .rounded(8)
            .width(1)
            .color(Color::from_rgb8(32, 50, 48)),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.35),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 24.0,
        },
        snap: false,
    }
}

pub(super) fn list_item_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(21, 27, 36))
        .border(Border::default().rounded(6).width(1).color(BORDER))
}

pub(super) fn selected_row_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(23, 44, 42))
        .border(Border::default().rounded(6).width(1).color(ACCENT))
}

pub(super) fn transparent_style(_: &Theme) -> container::Style {
    container::Style::default()
}

pub(super) fn accent_block_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(ACCENT)
        .border(Border::default().rounded(8))
}

pub(super) fn badge_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(34, 47, 59))
        .border(Border::default().rounded(8).width(1).color(BORDER))
}

pub(super) fn tag_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(31, 51, 69))
        .border(
            Border::default()
                .rounded(999)
                .width(1)
                .color(Color::from_rgb8(70, 105, 132)),
        )
}

pub(super) fn active_tab_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(28, 43, 49))
        .border(Border::default().rounded(8).width(1).color(ACCENT))
}

pub(super) fn quiet_tab_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(SURFACE)
        .border(Border::default().rounded(8).width(1).color(BORDER))
}

pub(super) fn pill_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(20, 26, 35))
        .border(Border::default().rounded(999).width(1).color(BORDER))
}

pub(super) fn rule_style(_: &Theme) -> container::Style {
    container::Style::default().background(BORDER)
}

pub(super) fn error_banner_style(_: &Theme) -> container::Style {
    container::Style::default()
        .background(Color::from_rgb8(78, 40, 32))
        .border(
            Border::default()
                .width(1)
                .color(Color::from_rgb8(144, 68, 48)),
        )
}

fn elevated_style(background: Color, radius: u8) -> container::Style {
    container::Style {
        text_color: Some(TEXT_SOFT),
        background: Some(Background::Color(background)),
        border: Border::default().rounded(radius).width(1).color(BORDER),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.20),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 14.0,
        },
        snap: false,
    }
}

pub(super) fn primary_button_style(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb8(61, 215, 160),
        _ => ACCENT,
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::from_rgb8(3, 20, 16),
        border: Border::default().rounded(6),
        shadow: Shadow::default(),
        snap: false,
    }
}

pub(super) fn ghost_button_style(_: &Theme, status: button::Status) -> button::Style {
    let bg = match status {
        button::Status::Hovered => Color::from_rgb8(34, 45, 58),
        _ => Color::from_rgb8(23, 30, 40),
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: TEXT_SOFT,
        border: Border::default().rounded(6).width(1).color(BORDER),
        shadow: Shadow::default(),
        snap: false,
    }
}

pub(super) fn danger_button_style(_: &Theme, _: button::Status) -> button::Style {
    button::Style {
        background: Some(Background::Color(Color::from_rgb8(74, 35, 35))),
        text_color: Color::from_rgb8(255, 196, 196),
        border: Border::default()
            .rounded(6)
            .width(1)
            .color(Color::from_rgb8(129, 56, 56)),
        shadow: Shadow::default(),
        snap: false,
    }
}

pub(super) fn flat_button_style(_: &Theme, _: button::Status) -> button::Style {
    button::Style {
        background: None,
        text_color: TEXT_SOFT,
        border: Border::default(),
        shadow: Shadow::default(),
        snap: false,
    }
}

pub(super) fn input_style(_: &Theme, status: text_input::Status) -> text_input::Style {
    let border_color = match status {
        text_input::Status::Focused { .. } => ACCENT,
        text_input::Status::Hovered => BLUE,
        _ => BORDER,
    };

    text_input::Style {
        background: Background::Color(Color::from_rgb8(12, 17, 24)),
        border: Border::default().rounded(6).width(1).color(border_color),
        icon: TEXT_MUTED,
        placeholder: TEXT_SUBTLE,
        value: TEXT_STRONG,
        selection: Color::from_rgb8(42, 94, 78),
    }
}

pub(super) fn terminal_input_style(_: &Theme, status: text_input::Status) -> text_input::Style {
    let border_color = match status {
        text_input::Status::Focused { .. } => ACCENT,
        _ => Color::from_rgb8(26, 47, 43),
    };

    text_input::Style {
        background: Background::Color(Color::from_rgb8(5, 12, 13)),
        border: Border::default().rounded(6).width(1).color(border_color),
        icon: TERMINAL_DIM,
        placeholder: TERMINAL_DIM,
        value: TERMINAL_TEXT,
        selection: Color::from_rgb8(25, 82, 62),
    }
}
