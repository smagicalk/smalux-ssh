use super::*;
use crate::model::{GroupId, HostGroup};

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
fn quick_host_group_message_updates_group_only() {
    let mut state = AppState::default();
    let group_id = GroupId(Uuid::new_v4());

    let outcome = state.apply(Message::SelectQuickHostGroup {
        group_id: Some(group_id),
    });

    assert!(outcome.changed());
    assert_eq!(state.ui.quick_host.group_id, Some(group_id));
    assert_eq!(state.storage.group_count(), 0);
}

#[test]
fn open_create_group_dialog_sets_parent_and_opens_dialog() {
    let mut state = AppState::default();
    let parent_id = GroupId(Uuid::new_v4());
    state.storage.upsert_group(HostGroup {
        id: parent_id,
        name: "production".to_owned(),
        parent_id: None,
    });
    state.ui.workspace.create_host_dialog_open = true;
    state.ui.quick_group.name = "stale".to_owned();

    let outcome = state.apply(Message::OpenCreateGroupDialog {
        parent_id: Some(parent_id),
    });

    assert!(outcome.changed());
    assert!(state.ui.workspace.create_group_dialog_open);
    assert!(!state.ui.workspace.create_host_dialog_open);
    assert_eq!(state.ui.quick_group.parent_id, Some(parent_id));
    assert_eq!(state.ui.quick_group.name, "");
    assert_eq!(state.storage.group_count(), 1);
}

#[test]
fn create_group_parent_dialog_confirms_into_group_dialog() {
    let mut state = AppState::default();
    let parent_id = GroupId(Uuid::new_v4());
    state.storage.upsert_group(HostGroup {
        id: parent_id,
        name: "production".to_owned(),
        parent_id: None,
    });

    let open = state.apply(Message::OpenCreateGroupParentDialog { parent_id: None });
    let select = state.apply(Message::SelectCreateGroupParent {
        parent_id: Some(parent_id),
    });
    let confirm = state.apply(Message::ConfirmCreateGroupParent);

    assert!(open.changed());
    assert!(select.changed());
    assert!(confirm.changed());
    assert!(!state.ui.workspace.create_group_parent_dialog_open);
    assert!(state.ui.workspace.create_group_dialog_open);
    assert_eq!(state.ui.quick_group.parent_id, Some(parent_id));
    assert_eq!(state.ui.workspace.pending_create_group_parent_id, None);
}

#[test]
fn save_quick_group_creates_group_and_resets_form() {
    let mut state = AppState::default();
    let parent_id = GroupId(Uuid::new_v4());
    state.storage.upsert_group(HostGroup {
        id: parent_id,
        name: "production".to_owned(),
        parent_id: None,
    });
    state.apply(Message::OpenCreateGroupDialog {
        parent_id: Some(parent_id),
    });
    state.apply(Message::UpdateQuickGroupName {
        name: " api ".to_owned(),
    });

    let outcome = state.apply(Message::SaveQuickGroup);

    assert!(outcome.changed());
    assert_eq!(state.storage.group_count(), 2);
    let saved = state
        .storage
        .groups
        .iter()
        .find(|group| group.parent_id == Some(parent_id))
        .expect("child group should be inserted");
    assert_eq!(saved.name, "api");
    assert_eq!(state.ui.quick_group.name, "");
    assert_eq!(state.ui.quick_group.parent_id, None);
    assert!(!state.ui.workspace.create_group_dialog_open);
}

#[test]
fn select_quick_group_parent_updates_group_draft_only() {
    let mut state = AppState::default();
    let parent_id = GroupId(Uuid::new_v4());
    state.storage.upsert_group(HostGroup {
        id: parent_id,
        name: "production".to_owned(),
        parent_id: None,
    });
    state.apply(Message::OpenCreateGroupDialog { parent_id: None });

    let outcome = state.apply(Message::SelectQuickGroupParent {
        parent_id: Some(parent_id),
    });

    assert!(outcome.changed());
    assert_eq!(state.ui.quick_group.parent_id, Some(parent_id));
    assert_eq!(state.storage.group_count(), 1);
    assert!(state.ui.workspace.create_group_dialog_open);
}

#[test]
fn save_quick_group_rejects_empty_name_without_side_effects() {
    let mut state = AppState::default();
    state.apply(Message::OpenCreateGroupDialog { parent_id: None });
    state.apply(Message::UpdateQuickGroupName {
        name: "  ".to_owned(),
    });

    let outcome = state.apply(Message::SaveQuickGroup);

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert_eq!(state.storage.group_count(), 0);
    assert!(state.ui.workspace.create_group_dialog_open);
}

#[test]
fn open_create_group_dialog_rejects_missing_parent() {
    let mut state = AppState::default();
    let missing_parent_id = GroupId(Uuid::new_v4());

    let outcome = state.apply(Message::OpenCreateGroupDialog {
        parent_id: Some(missing_parent_id),
    });

    assert!(outcome.changed());
    assert!(outcome.error.is_some());
    assert!(!state.ui.workspace.create_group_dialog_open);
    assert_eq!(state.ui.quick_group.parent_id, None);
    assert_eq!(state.storage.group_count(), 0);
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
    state
        .ui
        .set_quick_host_field(QuickHostDraftField::IconKey, "cloud".to_owned());

    let outcome = state.apply(Message::SaveQuickHost);

    assert!(outcome.changed());
    assert_eq!(state.storage.host_count(), 1);
    assert_eq!(state.storage.hosts[0].name, "prod.example.com");
    assert_eq!(state.storage.hosts[0].icon_key, "cloud");
    assert_eq!(state.storage.hosts[0].tags, vec!["prod", "linux"]);
    assert_eq!(state.storage.hosts[0].group_id, None);
    assert_eq!(state.ui.quick_host.address, "");
    assert_eq!(state.ui.quick_host.port, "22");
    assert_eq!(state.backend_commands.pending_count(), 0);
}

#[test]
fn save_quick_host_accepts_localized_tag_separators() {
    let mut state = AppState::default();
    state
        .ui
        .set_quick_host_field(QuickHostDraftField::Address, "prod.example.com".to_owned());
    state
        .ui
        .set_quick_host_field(QuickHostDraftField::Username, "deploy".to_owned());
    state.ui.set_quick_host_field(
        QuickHostDraftField::Tags,
        "prod，api、east；blue".to_owned(),
    );

    let outcome = state.apply(Message::SaveQuickHost);

    assert!(outcome.changed());
    assert_eq!(
        state.storage.hosts[0].tags,
        vec!["prod", "api", "east", "blue"]
    );
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

#[test]
fn open_edit_host_dialog_prefills_existing_host() {
    let mut state = AppState::default();
    let host = editable_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    let outcome = state.apply(Message::OpenEditHostDialog { host_id });

    assert!(outcome.changed());
    assert!(state.ui.workspace.create_host_dialog_open);
    assert_eq!(state.ui.quick_host.editing_host_id, Some(host_id));
    assert_eq!(state.ui.quick_host.name, "prod");
    assert_eq!(state.ui.quick_host.address, "prod.example.com");
    assert_eq!(state.ui.quick_host.port, "2202");
    assert_eq!(state.ui.quick_host.username, "deploy");
    assert_eq!(state.ui.quick_host.icon_key, "server");
    assert_eq!(state.ui.quick_host.tags, "prod, linux");
    assert_eq!(state.ui.quick_host.group_id, None);
    assert!(matches!(
        state.ui.quick_host.auth.kind,
        QuickHostAuthKind::Key
    ));
    assert_eq!(state.ui.quick_host.auth.private_key_ref, "key:prod");
}

#[test]
fn open_edit_host_dialog_prefills_existing_group_and_allows_reassignment() {
    let mut state = AppState::default();
    let original_group_id = GroupId(Uuid::new_v4());
    let target_group_id = GroupId(Uuid::new_v4());
    state.storage.upsert_group(HostGroup {
        id: original_group_id,
        name: "production".to_owned(),
        parent_id: None,
    });
    state.storage.upsert_group(HostGroup {
        id: target_group_id,
        name: "staging".to_owned(),
        parent_id: None,
    });
    let mut host = editable_host();
    host.group_id = Some(original_group_id);
    let host_id = host.id;
    state.storage.upsert_host(host);

    let outcome = state.apply(Message::OpenEditHostDialog { host_id });

    assert!(outcome.changed());
    assert_eq!(state.ui.quick_host.group_id, Some(original_group_id));

    state.apply(Message::SelectQuickHostGroup {
        group_id: Some(target_group_id),
    });
    let outcome = state.apply(Message::SaveQuickHost);

    assert!(outcome.changed());
    assert_eq!(state.storage.hosts[0].group_id, Some(target_group_id));
}

#[test]
fn save_quick_host_updates_existing_host_and_preserves_hidden_fields() {
    let mut state = AppState::default();
    let host = editable_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenEditHostDialog { host_id });
    state.apply(Message::UpdateQuickHostDraft {
        field: QuickHostDraftField::Address,
        value: "new.example.com".to_owned(),
    });
    state.apply(Message::UpdateQuickHostDraft {
        field: QuickHostDraftField::Tags,
        value: "prod, blue".to_owned(),
    });
    state.apply(Message::UpdateQuickHostDraft {
        field: QuickHostDraftField::IconKey,
        value: "database".to_owned(),
    });

    let outcome = state.apply(Message::SaveQuickHost);

    assert!(outcome.changed());
    assert_eq!(state.storage.host_count(), 1);
    let saved = &state.storage.hosts[0];
    assert_eq!(saved.id, host_id);
    assert_eq!(saved.address, "new.example.com");
    assert_eq!(saved.icon_key, "database");
    assert_eq!(saved.tags, vec!["prod", "blue"]);
    assert_eq!(saved.group_id, None);
    assert_eq!(
        saved
            .theme_override
            .as_ref()
            .map(|theme| theme.name.as_str()),
        Some("Host Dark")
    );
    assert_eq!(state.ui.quick_host.editing_host_id, None);
    assert!(!state.ui.workspace.create_host_dialog_open);
}

#[test]
fn save_quick_host_preserves_selected_group_for_new_hosts() {
    let mut state = AppState::default();
    let group_id = GroupId(Uuid::new_v4());
    state.storage.upsert_group(HostGroup {
        id: group_id,
        name: "production".to_owned(),
        parent_id: None,
    });
    state.apply(Message::SelectQuickHostGroup {
        group_id: Some(group_id),
    });
    state
        .ui
        .set_quick_host_field(QuickHostDraftField::Address, "prod.example.com".to_owned());
    state
        .ui
        .set_quick_host_field(QuickHostDraftField::Username, "deploy".to_owned());

    let outcome = state.apply(Message::SaveQuickHost);

    assert!(outcome.changed());
    assert_eq!(state.storage.hosts[0].group_id, Some(group_id));
}

#[test]
fn open_create_host_dialog_in_group_preselects_group() {
    let mut state = AppState::default();
    let group_id = GroupId(Uuid::new_v4());
    state.storage.upsert_group(HostGroup {
        id: group_id,
        name: "production".to_owned(),
        parent_id: None,
    });
    state.ui.quick_host.name = "stale".to_owned();

    let outcome = state.apply(Message::OpenCreateHostDialogInGroup {
        group_id: Some(group_id),
    });

    assert!(outcome.changed());
    assert!(state.ui.workspace.create_host_dialog_open);
    assert_eq!(state.ui.quick_host.group_id, Some(group_id));
    assert_eq!(state.ui.quick_host.name, "");
}

#[test]
fn duplicate_host_copies_saved_host_with_new_name_and_id() {
    let mut state = AppState::default();
    let host = editable_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    let outcome = state.apply(Message::DuplicateHost { host_id });

    assert!(outcome.changed());
    assert_eq!(state.storage.host_count(), 2);
    let original = state
        .storage
        .hosts
        .iter()
        .find(|host| host.id == host_id)
        .expect("original host should stay");
    let duplicate = state
        .storage
        .hosts
        .iter()
        .find(|host| host.id != host_id)
        .expect("duplicate host should be inserted");
    assert_eq!(duplicate.name, "prod 复制");
    assert_ne!(duplicate.id, original.id);
    assert_eq!(duplicate.address, original.address);
    assert_eq!(duplicate.port, original.port);
    assert_eq!(duplicate.auth, original.auth);
    assert_eq!(duplicate.icon_key, original.icon_key);
    assert_eq!(duplicate.tags, original.tags);
}

#[test]
fn request_remove_host_only_opens_confirmation() {
    let mut state = AppState::default();
    let host = editable_host();
    let host_id = host.id;
    state.storage.upsert_host(host);

    let outcome = state.apply(Message::RequestRemoveHost { host_id });

    assert!(outcome.changed());
    assert_eq!(state.storage.host_count(), 1);
    assert_eq!(state.ui.workspace.pending_delete_host_id, Some(host_id));
    assert_eq!(state.backend_commands.pending_count(), 0);
}

#[test]
fn cancel_remove_host_keeps_saved_host() {
    let mut state = AppState::default();
    let host = editable_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::RequestRemoveHost { host_id });

    let outcome = state.apply(Message::CancelRemoveHost);

    assert!(outcome.changed());
    assert_eq!(state.storage.host_count(), 1);
    assert_eq!(state.ui.workspace.pending_delete_host_id, None);
}

#[test]
fn confirm_remove_host_deletes_pending_saved_host() {
    let mut state = AppState::default();
    let host = editable_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::RequestRemoveHost { host_id });

    let outcome = state.apply(Message::ConfirmRemoveHost);

    assert!(outcome.changed());
    assert_eq!(state.storage.host_count(), 0);
    assert_eq!(state.ui.workspace.pending_delete_host_id, None);
    assert_eq!(state.backend_commands.pending_count(), 0);
}

#[test]
fn request_remove_group_only_opens_confirmation() {
    let mut state = AppState::default();
    let group_id = GroupId(Uuid::new_v4());
    state.storage.upsert_group(HostGroup {
        id: group_id,
        name: "production".to_owned(),
        parent_id: None,
    });

    let outcome = state.apply(Message::RequestRemoveGroup { group_id });

    assert!(outcome.changed());
    assert_eq!(state.storage.group_count(), 1);
    assert_eq!(state.ui.workspace.pending_delete_group_id, Some(group_id));
}

#[test]
fn confirm_remove_group_deletes_group_children_and_hosts() {
    let mut state = AppState::default();
    let parent_id = GroupId(Uuid::new_v4());
    let child_id = GroupId(Uuid::new_v4());
    state.storage.upsert_group(HostGroup {
        id: parent_id,
        name: "production".to_owned(),
        parent_id: None,
    });
    state.storage.upsert_group(HostGroup {
        id: child_id,
        name: "api".to_owned(),
        parent_id: Some(parent_id),
    });
    let mut host = editable_host();
    host.group_id = Some(child_id);
    state.storage.upsert_host(host);
    state.apply(Message::RequestRemoveGroup {
        group_id: parent_id,
    });

    let outcome = state.apply(Message::ConfirmRemoveGroup);

    assert!(outcome.changed());
    assert_eq!(state.storage.group_count(), 0);
    assert_eq!(state.storage.host_count(), 0);
    assert_eq!(state.ui.workspace.pending_delete_group_id, None);
}

fn editable_host() -> Host {
    Host {
        id: HostId(Uuid::new_v4()),
        name: "prod".to_owned(),
        group_id: None,
        icon_key: "server".to_owned(),
        tags: vec!["prod".to_owned(), "linux".to_owned()],
        address: "prod.example.com".to_owned(),
        port: 2202,
        auth: AuthProfile::Key {
            username: "deploy".to_owned(),
            key: SecretRef("key:prod".to_owned()),
            passphrase: Some(SecretRef("passphrase:prod".to_owned())),
        },
        proxy: None,
        jumps: Vec::new(),
        theme_override: Some(ThemeProfile {
            name: "Host Dark".to_owned(),
            font_family: "JetBrains Mono".to_owned(),
            font_size: 14.0,
        }),
        background_override: None,
    }
}
