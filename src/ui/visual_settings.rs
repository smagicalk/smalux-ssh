//! 全局视觉配置面板。

use iced::{
    Element, Length,
    widget::{button, checkbox, column, row, text, text_input},
};

use crate::model::{AppState, Message, VisualSettingsDraftField};

/// 渲染全局主题和背景设置。
pub fn view(state: &AppState) -> Element<'_, Message> {
    let draft = &state.ui.visual_settings;

    column![
        text("Visual").size(22),
        row![
            visual_input(
                "theme name",
                &draft.theme_name,
                VisualSettingsDraftField::ThemeName,
            ),
            visual_input(
                "font family",
                &draft.font_family,
                VisualSettingsDraftField::FontFamily,
            ),
            visual_input(
                "font size",
                &draft.font_size,
                VisualSettingsDraftField::FontSize,
            ),
        ]
        .spacing(8),
        row![
            checkbox(draft.background_enabled)
                .label("Background enabled")
                .on_toggle(|enabled| Message::SetVisualBackgroundEnabled { enabled }),
            visual_input(
                "background sources",
                &draft.background_sources,
                VisualSettingsDraftField::BackgroundSources,
            ),
        ]
        .spacing(8),
        row![
            visual_input(
                "rotation secs",
                &draft.rotation_interval_secs,
                VisualSettingsDraftField::RotationIntervalSecs,
            ),
            visual_input("opacity", &draft.opacity, VisualSettingsDraftField::Opacity),
            visual_input("blur", &draft.blur, VisualSettingsDraftField::Blur),
            button("Apply").on_press(Message::ApplyVisualSettings),
        ]
        .spacing(8),
    ]
    .spacing(8)
    .into()
}

fn visual_input<'a>(
    placeholder: &'a str,
    value: &'a str,
    field: VisualSettingsDraftField,
) -> Element<'a, Message> {
    text_input(placeholder, value)
        .on_input(move |value| Message::UpdateVisualSettingsDraft { field, value })
        .width(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_settings_view_accepts_default_state() {
        let state = AppState::default();

        let _element = view(&state);
    }

    #[test]
    fn visual_settings_view_accepts_enabled_background() {
        let mut state = AppState::default();
        state.ui.set_visual_background_enabled(true);
        state.ui.set_visual_settings_field(
            VisualSettingsDraftField::BackgroundSources,
            "wallpapers/a.jpg".to_owned(),
        );

        let _element = view(&state);
    }
}
