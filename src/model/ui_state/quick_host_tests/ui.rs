use crate::model::ui_state::{QuickHostAuthField, QuickHostAuthKind, QuickHostDraftField, UiState};

#[test]
fn ui_state_quick_host_messages_update_form_only() {
    let mut state = UiState::default();

    state.set_quick_host_field(QuickHostDraftField::Address, "example.com");
    state.set_quick_host_field(QuickHostDraftField::Username, "ops");
    state.set_quick_host_auth_kind(QuickHostAuthKind::Password);
    state.set_quick_host_auth_field(QuickHostAuthField::PasswordSecretRef, "password:ops");
    state.reset_quick_host();

    assert_eq!(state.quick_host.address, "");
    assert_eq!(state.quick_host.username, "");
    assert_eq!(state.quick_host.port, "22");
    assert!(matches!(
        state.quick_host.auth.kind,
        QuickHostAuthKind::Agent
    ));
}
