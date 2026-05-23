use super::super::{UiState, VisualSettingsDraftField};
use super::common::{background, host_id, theme};

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
