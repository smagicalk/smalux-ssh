use super::*;

#[test]
fn visual_settings_messages_update_draft_and_apply_config() {
    let mut state = AppState::default();

    state.apply(Message::UpdateVisualSettingsDraft {
        field: VisualSettingsDraftField::ThemeName,
        value: "Solarized Dark".to_owned(),
    });
    state.apply(Message::UpdateVisualSettingsDraft {
        field: VisualSettingsDraftField::FontFamily,
        value: "Maple Mono".to_owned(),
    });
    state.apply(Message::UpdateVisualSettingsDraft {
        field: VisualSettingsDraftField::FontSize,
        value: "16".to_owned(),
    });
    state.apply(Message::SetVisualBackgroundEnabled { enabled: true });
    state.apply(Message::UpdateVisualSettingsDraft {
        field: VisualSettingsDraftField::BackgroundSources,
        value: "wallpapers/a.jpg, url:https://example.com/b.jpg".to_owned(),
    });
    state.apply(Message::UpdateVisualSettingsDraft {
        field: VisualSettingsDraftField::RotationIntervalSecs,
        value: "120".to_owned(),
    });
    state.apply(Message::UpdateVisualSettingsDraft {
        field: VisualSettingsDraftField::Opacity,
        value: "0.4".to_owned(),
    });
    state.apply(Message::UpdateVisualSettingsDraft {
        field: VisualSettingsDraftField::Blur,
        value: "12".to_owned(),
    });

    let outcome = state.apply(Message::ApplyVisualSettings);

    assert!(outcome.changed());
    assert_eq!(state.config.theme.name, "Solarized Dark");
    assert_eq!(state.config.theme.font_family, "Maple Mono");
    assert_eq!(state.config.theme.font_size, 16.0);
    assert!(state.config.background.enabled);
    assert_eq!(state.config.background.rotation_interval_secs, 120);
    assert_eq!(state.config.background.opacity, 0.4);
    assert_eq!(state.config.background.blur, 12.0);
    assert_eq!(
        state.config.background.sources,
        vec![
            ImageSource::LocalPath("wallpapers/a.jpg".to_owned()),
            ImageSource::Url("https://example.com/b.jpg".to_owned()),
        ]
    );
    assert_eq!(state.storage.app_config, state.config);
}
