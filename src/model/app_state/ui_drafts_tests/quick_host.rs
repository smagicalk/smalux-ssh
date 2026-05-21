use super::*;

#[test]
fn quick_host_draft_message_updates_form_only() {
    let mut state = AppState::default();

    let outcome = state.apply(Message::UpdateQuickHostDraft {
        field: QuickHostDraftField::Address,
        value: "example.com".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(state.ui.quick_host.address, "example.com");
    assert_eq!(state.storage.host_count(), 0);
}

#[test]
fn quick_host_auth_messages_update_auth_draft_only() {
    let mut state = AppState::default();

    let kind_outcome = state.apply(Message::UpdateQuickHostAuthKind {
        kind: QuickHostAuthKind::Password,
    });
    let field_outcome = state.apply(Message::UpdateQuickHostAuthField {
        field: QuickHostAuthField::PasswordSecretRef,
        value: "password:root".to_owned(),
    });

    assert!(kind_outcome.changed());
    assert!(field_outcome.changed());
    assert!(matches!(
        state.ui.quick_host.auth.kind,
        QuickHostAuthKind::Password
    ));
    assert_eq!(
        state.ui.quick_host.auth.password_secret_ref,
        "password:root"
    );
    assert_eq!(state.storage.host_count(), 0);
}

#[test]
fn save_quick_host_creates_agent_host_and_resets_form() {
    let mut state = AppState::default();
    state
        .ui
        .set_quick_host_field(QuickHostDraftField::Address, "prod.example.com".to_owned());
    state
        .ui
        .set_quick_host_field(QuickHostDraftField::Username, "deploy".to_owned());
    state
        .ui
        .set_quick_host_field(QuickHostDraftField::Tags, "prod,linux".to_owned());

    let outcome = state.apply(Message::SaveQuickHost);

    assert!(outcome.changed());
    assert_eq!(state.storage.host_count(), 1);
    assert_eq!(state.storage.hosts[0].name, "prod.example.com");
    assert_eq!(state.storage.hosts[0].tags, vec!["prod", "linux"]);
    assert_eq!(state.ui.quick_host.address, "");
    assert_eq!(state.ui.quick_host.port, "22");
    assert_eq!(state.backend_commands.pending_count(), 0);
}

#[test]
fn save_quick_host_honors_selected_password_auth() {
    let mut state = AppState::default();
    state
        .ui
        .set_quick_host_field(QuickHostDraftField::Address, "root.example.com".to_owned());
    state
        .ui
        .set_quick_host_field(QuickHostDraftField::Username, "root".to_owned());
    state
        .ui
        .set_quick_host_auth_kind(QuickHostAuthKind::Password);
    state.ui.set_quick_host_auth_field(
        QuickHostAuthField::PasswordSecretRef,
        "password:root".to_owned(),
    );

    let outcome = state.apply(Message::SaveQuickHost);

    assert!(outcome.changed());
    assert_eq!(state.storage.host_count(), 1);
    assert!(matches!(
        &state.storage.hosts[0].auth,
        AuthProfile::Password {
            username,
            secret: SecretRef(secret_ref),
        } if username == "root" && secret_ref == "password:root"
    ));
}

#[test]
fn save_quick_host_rejects_invalid_form_without_side_effects() {
    let mut state = AppState::default();

    let outcome = state.apply(Message::SaveQuickHost);

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert_eq!(state.storage.host_count(), 0);
    assert_eq!(state.backend_commands.pending_count(), 0);
}
