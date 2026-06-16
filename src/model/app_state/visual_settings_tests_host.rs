use super::*;

#[test]
fn host_visual_settings_apply_and_clear_host_overrides() {
    let mut state = desktop_state();
    let host = sample_host();
    let host_id = host.id;
    state.core.storage.upsert_host(host);

    state.apply_message(Message::UpdateHostVisualSettingsDraft {
        host_id,
        field: VisualSettingsDraftField::ThemeName,
        value: "Prod Dark".to_owned(),
    });
    state.apply_message(Message::SetHostVisualBackgroundEnabled {
        host_id,
        enabled: true,
    });
    state.apply_message(Message::UpdateHostVisualSettingsDraft {
        host_id,
        field: VisualSettingsDraftField::BackgroundSources,
        value: "wallpapers/prod.jpg".to_owned(),
    });

    let apply_outcome = state.apply_message(Message::ApplyHostVisualSettings { host_id });

    assert!(apply_outcome.changed());
    assert_eq!(
        state.core.storage.hosts[0]
            .theme_override
            .as_ref()
            .map(|theme| theme.name.as_str()),
        Some("Prod Dark")
    );
    assert_eq!(
        state.core.storage.hosts[0]
            .background_override
            .as_ref()
            .map(|background| background.sources.clone()),
        Some(vec![ImageSource::LocalPath(
            "wallpapers/prod.jpg".to_owned()
        )])
    );
    assert!(state.ui.host_visual_settings_for(host_id).is_none());

    let clear_outcome = state.apply_message(Message::ClearHostVisualSettings { host_id });

    assert!(clear_outcome.changed());
    assert!(state.core.storage.hosts[0].theme_override.is_none());
    assert!(state.core.storage.hosts[0].background_override.is_none());
}
