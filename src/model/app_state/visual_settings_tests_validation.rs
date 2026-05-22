use super::*;

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
fn host_visual_settings_report_missing_host() {
    let mut state = AppState::default();
    let missing_host_id = HostId(Uuid::new_v4());

    let outcome = state.apply(Message::ApplyHostVisualSettings {
        host_id: missing_host_id,
    });

    assert!(outcome.error.is_some());
}
