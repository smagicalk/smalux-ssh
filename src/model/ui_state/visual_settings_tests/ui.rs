use super::super::{UiState, VisualSettingsDraftField};

#[test]
fn ui_state_visual_settings_messages_update_draft_only() {
    let mut state = UiState::default();

    state.set_visual_settings_field(VisualSettingsDraftField::ThemeName, "Solarized Dark");
    state.set_visual_settings_field(VisualSettingsDraftField::FontFamily, "Maple Mono");
    state.set_visual_settings_field(VisualSettingsDraftField::FontSize, "16");
    state.set_visual_background_enabled(true);
    state.set_visual_settings_field(
        VisualSettingsDraftField::BackgroundSources,
        "wallpapers/a.jpg, url:https://example.com/b.jpg",
    );
    state.set_visual_settings_field(VisualSettingsDraftField::RotationIntervalSecs, "120");
    state.set_visual_settings_field(VisualSettingsDraftField::Opacity, "0.4");
    state.set_visual_settings_field(VisualSettingsDraftField::Blur, "12");

    assert_eq!(state.visual_settings.theme_name, "Solarized Dark");
    assert_eq!(state.visual_settings.font_family, "Maple Mono");
    assert_eq!(state.visual_settings.font_size, "16");
    assert!(state.visual_settings.background_enabled);
    assert_eq!(
        state.visual_settings.background_sources,
        "wallpapers/a.jpg, url:https://example.com/b.jpg"
    );
    assert_eq!(state.visual_settings.rotation_interval_secs, "120");
    assert_eq!(state.visual_settings.opacity, "0.4");
    assert_eq!(state.visual_settings.blur, "12");
}
