use super::*;

#[test]
fn invalid_visual_settings_report_error_without_changing_config() {
    let mut state = desktop_state();
    let before = state.core.config.clone();
    state.apply_message(Message::UpdateVisualSettingsDraft {
        field: VisualSettingsDraftField::FontSize,
        value: "zero".to_owned(),
    });

    let outcome = state.apply_message(Message::ApplyVisualSettings);

    assert!(outcome.error.is_some());
    assert_eq!(state.core.config, before);
    assert_eq!(state.core.storage.app_config, before);
}

#[test]
fn host_visual_settings_report_missing_host() {
    let mut state = desktop_state();
    let missing_host_id = HostId(Uuid::new_v4());

    let outcome = state.apply_message(Message::ApplyHostVisualSettings {
        host_id: missing_host_id,
    });

    assert!(outcome.error.is_some());
}
