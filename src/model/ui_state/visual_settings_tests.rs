use super::*;
use crate::model::ImageSource;
use uuid::Uuid;

fn host_id() -> HostId {
    HostId(Uuid::new_v4())
}

fn theme() -> ThemeProfile {
    ThemeProfile {
        name: "Default Dark".to_owned(),
        font_family: "JetBrains Mono".to_owned(),
        font_size: 14.0,
    }
}

fn background() -> BackgroundProfile {
    BackgroundProfile {
        enabled: false,
        sources: Vec::new(),
        rotation_interval_secs: 300,
        opacity: 0.18,
        blur: 8.0,
    }
}

#[test]
fn draft_round_trips_profiles() {
    let theme = ThemeProfile {
        name: "Solarized".to_owned(),
        font_family: "Maple Mono".to_owned(),
        font_size: 15.5,
    };
    let background = BackgroundProfile {
        enabled: true,
        sources: vec![
            ImageSource::LocalPath("wallpapers/a.jpg".to_owned()),
            ImageSource::Url("https://example.com/b.jpg".to_owned()),
        ],
        rotation_interval_secs: 120,
        opacity: 0.4,
        blur: 12.0,
    };

    let draft = VisualSettingsDraft::from_profiles(&theme, &background);
    let rebuilt_theme = draft
        .build_theme_profile(&ThemeProfile {
            name: String::new(),
            font_family: String::new(),
            font_size: 14.0,
        })
        .expect("主题草稿应该可以还原");
    let rebuilt_background = draft
        .build_background_profile(&BackgroundProfile {
            enabled: false,
            sources: Vec::new(),
            rotation_interval_secs: 300,
            opacity: 0.18,
            blur: 8.0,
        })
        .expect("背景草稿应该可以还原");

    assert_eq!(rebuilt_theme, theme);
    assert_eq!(rebuilt_background, background.normalized());
}

#[test]
fn draft_reports_invalid_background_sources() {
    let draft = VisualSettingsDraft {
        background_sources: "url:".to_owned(),
        ..VisualSettingsDraft::default()
    };

    assert!(matches!(
        draft.build_background_profile(&BackgroundProfile {
            enabled: false,
            sources: Vec::new(),
            rotation_interval_secs: 300,
            opacity: 0.18,
            blur: 8.0,
        }),
        Err(VisualSettingsDraftError::InvalidBackgroundSource(_))
    ));
}

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

#[test]
fn ui_state_host_visual_settings_drafts_are_scoped_per_host() {
    let mut ui = UiState::default();
    let first = host_id();
    let second = host_id();

    ui.set_host_visual_settings_field(
        first,
        VisualSettingsDraftField::ThemeName,
        "Prod Dark",
        &theme(),
        &background(),
    );
    ui.set_host_visual_background_enabled(second, true, &theme(), &background());

    assert_eq!(
        ui.host_visual_settings_for(first)
            .map(|draft| draft.theme_name.as_str()),
        Some("Prod Dark")
    );
    assert_eq!(
        ui.host_visual_settings_for(second)
            .map(|draft| draft.background_enabled),
        Some(true)
    );
    assert_eq!(ui.host_visual_settings_drafts.len(), 2);

    ui.clear_host_visual_settings_draft(first);
    assert!(ui.host_visual_settings_for(first).is_none());
    assert!(ui.host_visual_settings_for(second).is_some());
}
