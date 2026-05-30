use crate::backend::BackendEvent;
use crate::model::{
    HostId, QuickHostDraftField, SessionId, VisualSettingsDraftField, WorkspacePage,
};

use super::{Message, MessageDispatchTarget};

#[test]
fn classifies_representative_messages_by_dispatch_target() {
    assert_eq!(
        MessageDispatchTarget::for_message(&Message::UpdateVisualSettingsDraft {
            field: VisualSettingsDraftField::FontSize,
            value: "16".to_owned(),
        }),
        MessageDispatchTarget::Visual
    );
    assert_eq!(
        MessageDispatchTarget::for_message(&Message::SaveWorkspaceSnapshot),
        MessageDispatchTarget::Workspace
    );
    assert_eq!(
        MessageDispatchTarget::for_message(&Message::UpdateQuickHostDraft {
            field: QuickHostDraftField::Address,
            value: "example.com".to_owned(),
        }),
        MessageDispatchTarget::Ui
    );
    assert_eq!(
        MessageDispatchTarget::for_message(&Message::RemoveCredential {
            name: "password:root".to_owned(),
        }),
        MessageDispatchTarget::Storage
    );
    assert_eq!(
        MessageDispatchTarget::for_message(&Message::ActivateTerminalTab {
            session_id: SessionId(uuid::Uuid::nil()),
        }),
        MessageDispatchTarget::Session
    );
    assert_eq!(
        MessageDispatchTarget::for_message(&Message::RefreshSftp {
            host_id: HostId(uuid::Uuid::nil()),
        }),
        MessageDispatchTarget::Sftp
    );
    assert_eq!(
        MessageDispatchTarget::for_message(&Message::OpenShell {
            host_id: HostId(uuid::Uuid::nil()),
        }),
        MessageDispatchTarget::Launch
    );
    assert_eq!(
        MessageDispatchTarget::for_message(&Message::ReconnectShell {
            session_id: SessionId(uuid::Uuid::nil()),
        }),
        MessageDispatchTarget::Launch
    );
    assert_eq!(
        MessageDispatchTarget::for_message(&Message::RunCommandHistory {
            history_id: crate::model::CommandHistoryId(uuid::Uuid::nil()),
        }),
        MessageDispatchTarget::Snippet
    );
    assert_eq!(
        MessageDispatchTarget::for_message(&Message::BackendEventReceived(
            BackendEvent::Disconnected {
                session_id: SessionId(uuid::Uuid::nil()),
            },
        )),
        MessageDispatchTarget::Backend
    );
}

#[test]
fn workspace_page_message_stays_in_ui_target() {
    assert_eq!(
        MessageDispatchTarget::for_message(&Message::SetWorkspacePage {
            page: WorkspacePage::Hosts,
        }),
        MessageDispatchTarget::Ui
    );
}

#[test]
fn theme_profile_messages_stay_in_ui_target() {
    assert_eq!(
        MessageDispatchTarget::for_message(&Message::ApplyThemeProfile {
            name: "Imported".to_owned(),
        }),
        MessageDispatchTarget::Ui
    );
    assert_eq!(
        MessageDispatchTarget::for_message(&Message::RemoveThemeProfile {
            name: "Imported".to_owned(),
        }),
        MessageDispatchTarget::Ui
    );
}
