use super::*;
use crate::backend::BackendCommand;
use crate::model::{
    AuthProfile, Message, QuickHostAuthField, QuickHostAuthKind, QuickHostDraftField, SecretRef,
    SftpActionDraftField,
};
use uuid::Uuid;

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

#[test]
fn command_draft_message_updates_ui_state_only() {
    let mut state = AppState::default();
    let host_id = HostId(Uuid::new_v4());

    let outcome = state.apply(Message::UpdateHostCommandDraft {
        host_id,
        command: "whoami".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(state.ui.remote_command_for(host_id), "whoami");
    assert_eq!(state.backend_commands.pending_count(), 0);
}

#[test]
fn sftp_initial_dir_draft_message_updates_ui_state_only() {
    let mut state = AppState::default();
    let host_id = HostId(Uuid::new_v4());

    let outcome = state.apply(Message::UpdateHostSftpInitialDirDraft {
        host_id,
        initial_dir: "/etc".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(state.ui.sftp_initial_dir_for(host_id), "/etc");
    assert_eq!(state.sessions.tab_count(), 0);
}

#[test]
fn sftp_action_draft_message_updates_ui_state_only() {
    let mut state = AppState::default();
    let host_id = HostId(Uuid::new_v4());

    let outcome = state.apply(Message::UpdateSftpActionDraft {
        host_id,
        field: SftpActionDraftField::LocalPath,
        value: "C:/tmp/app.tar.gz".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(state.ui.sftp_local_path_for(host_id), "C:/tmp/app.tar.gz");
    assert_eq!(state.backend_commands.pending_count(), 0);
}

#[test]
fn terminal_input_draft_message_updates_ui_state_only() {
    let mut state = AppState::default();
    let session_id = SessionId(Uuid::new_v4());

    let outcome = state.apply(Message::UpdateTerminalInputDraft {
        session_id,
        input: "ls".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(state.ui.terminal_input_for(session_id), "ls");
    assert_eq!(state.backend_commands.pending_count(), 0);
}

#[test]
fn terminal_key_messages_edit_input_draft_without_backend_side_effects() {
    let mut state = AppState::default();
    let session_id = SessionId(Uuid::new_v4());

    state.apply(Message::AppendTerminalInputDraft {
        session_id,
        text: "ls".to_owned(),
    });
    state.apply(Message::AppendTerminalInputDraft {
        session_id,
        text: "\u{e001}".to_owned(),
    });
    state.apply(Message::BackspaceTerminalInputDraft { session_id });

    assert_eq!(state.ui.terminal_input_for(session_id), "l");
    assert_eq!(state.backend_commands.pending_count(), 0);
}

#[test]
fn local_terminal_input_is_visible_and_queues_on_enter() {
    let mut state = AppState::default();
    let session_id = crate::model::LOCAL_TERMINAL_SESSION_ID;

    let text = state.apply(Message::UpdateTerminalInputDraft {
        session_id,
        input: "echo smagicalssh-local".to_owned(),
    });
    assert!(text.changed());
    assert_eq!(
        state.ui.terminal_input_for(session_id),
        "echo smagicalssh-local"
    );

    let enter = state.apply(Message::SendTerminalInput { session_id });

    assert!(enter.changed());
    assert_eq!(state.backend_commands.pending_count(), 1);
    assert_eq!(state.ui.terminal_input_for(session_id), "");
    assert_eq!(
        state.terminal.tabs[0].buffer,
        vec![format!(
            "{} echo smagicalssh-local",
            crate::backend::LocalShellProfile::default_for_platform().prompt
        )]
    );
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::SendShellInput { session_id: queued_session_id, input })
            if *queued_session_id == session_id && input == "echo smagicalssh-local\n"
    ));
    assert_eq!(state.storage.command_history_count(), 1);
    assert_eq!(state.storage.command_history[0].host_id, None);
}

#[test]
fn local_terminal_starts_without_help_banner() {
    let mut state = AppState::default();
    let session_id = crate::model::LOCAL_TERMINAL_SESSION_ID;

    assert!(ui_drafts::ensure_local_terminal_tab(&mut state, session_id));
    assert!(!ui_drafts::ensure_local_terminal_tab(
        &mut state, session_id
    ));

    let tab = state
        .terminal
        .tabs
        .iter()
        .find(|tab| tab.session_id == session_id)
        .expect("local terminal tab should exist");
    assert_eq!(tab.title, crate::model::DEFAULT_LOCAL_TERMINAL_TITLE);
    assert!(tab.buffer.is_empty());
}

#[test]
fn local_terminal_empty_enter_queues_newline_without_history() {
    let mut state = AppState::default();
    let session_id = crate::model::LOCAL_TERMINAL_SESSION_ID;

    ui_drafts::ensure_local_terminal_tab(&mut state, session_id);
    state.apply(Message::UpdateTerminalInputDraft {
        session_id,
        input: String::new(),
    });

    let outcome = state.apply(Message::SendTerminalInput { session_id });

    assert!(outcome.changed());
    assert_eq!(state.ui.terminal_input_for(session_id), "");
    assert!(matches!(
        state.backend_commands.front(),
        Some(BackendCommand::SendShellInput { session_id: queued_session_id, input })
            if *queued_session_id == session_id && input == "\n"
    ));
    assert_eq!(state.storage.command_history_count(), 0);
}
