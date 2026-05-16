use super::*;
use crate::model::{AuthProfile, Host, HostId, ImageSource, Message};
use uuid::Uuid;

fn sample_host() -> Host {
    Host {
        id: HostId(Uuid::new_v4()),
        name: "production".to_owned(),
        group_id: None,
        tags: Vec::new(),
        address: "prod.example.com".to_owned(),
        port: 22,
        auth: AuthProfile::Agent {
            username: "deploy".to_owned(),
            key_hint: None,
        },
        proxy: None,
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    }
}

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

#[test]
fn invalid_visual_settings_report_error_without_changing_config() {
    let mut state = AppState::default();
    let before = state.config.clone();
    state.apply(Message::UpdateVisualSettingsDraft {
        field: VisualSettingsDraftField::FontSize,
        value: "zero".to_owned(),
    });

    let outcome = state.apply(Message::ApplyVisualSettings);

    assert!(outcome.error.is_some());
    assert_eq!(state.config, before);
    assert_eq!(state.storage.app_config, before);
}

#[test]
fn host_visual_settings_apply_and_clear_host_overrides() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    state.apply(Message::UpdateHostVisualSettingsDraft {
        host_id,
        field: VisualSettingsDraftField::ThemeName,
        value: "Prod Dark".to_owned(),
    });
    state.apply(Message::SetHostVisualBackgroundEnabled {
        host_id,
        enabled: true,
    });
    state.apply(Message::UpdateHostVisualSettingsDraft {
        host_id,
        field: VisualSettingsDraftField::BackgroundSources,
        value: "wallpapers/prod.jpg".to_owned(),
    });

    let apply_outcome = state.apply(Message::ApplyHostVisualSettings { host_id });

    assert!(apply_outcome.changed());
    assert_eq!(
        state.storage.hosts[0]
            .theme_override
            .as_ref()
            .map(|theme| theme.name.as_str()),
        Some("Prod Dark")
    );
    assert_eq!(
        state.storage.hosts[0]
            .background_override
            .as_ref()
            .map(|background| background.sources.clone()),
        Some(vec![ImageSource::LocalPath(
            "wallpapers/prod.jpg".to_owned()
        )])
    );
    assert!(state.ui.host_visual_settings_for(host_id).is_none());

    let clear_outcome = state.apply(Message::ClearHostVisualSettings { host_id });

    assert!(clear_outcome.changed());
    assert!(state.storage.hosts[0].theme_override.is_none());
    assert!(state.storage.hosts[0].background_override.is_none());
}

#[test]
fn host_visual_settings_report_missing_host() {
    let mut state = AppState::default();
    let missing_host_id = HostId(Uuid::new_v4());

    let outcome = state.apply(Message::ApplyHostVisualSettings {
        host_id: missing_host_id,
    });

    assert!(outcome.error.is_some());
}
