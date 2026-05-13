//! UI 输入草稿消息处理。

use crate::model::HostId;
use crate::model::QuickHostAuthField;
use crate::model::QuickHostAuthKind;
use crate::model::QuickHostDraftField;
use uuid::Uuid;

use super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 更新快速新增主机表单草稿。
    pub(super) fn update_quick_host_draft(
        &mut self,
        field: QuickHostDraftField,
        value: String,
    ) -> AppUpdateOutcome {
        self.ui.set_quick_host_field(field, value);
        draft_changed()
    }

    /// 更新快速新增主机认证方式。
    pub(super) fn update_quick_host_auth_kind(
        &mut self,
        kind: QuickHostAuthKind,
    ) -> AppUpdateOutcome {
        self.ui.set_quick_host_auth_kind(kind);
        draft_changed()
    }

    /// 更新快速新增主机认证字段。
    pub(super) fn update_quick_host_auth_field(
        &mut self,
        field: QuickHostAuthField,
        value: String,
    ) -> AppUpdateOutcome {
        self.ui.set_quick_host_auth_field(field, value);
        draft_changed()
    }

    /// 保存快速新增主机。
    pub(super) fn save_quick_host(&mut self) -> AppUpdateOutcome {
        let host_id = HostId(Uuid::new_v4());
        let host = match self.ui.quick_host.build_host(host_id) {
            Ok(host) => host,
            Err(error) => {
                return AppUpdateOutcome {
                    error: Some(format!("主机表单无效：{error}")),
                    ..AppUpdateOutcome::default()
                };
            }
        };

        self.storage.upsert_host(host);
        self.ui.reset_quick_host();
        draft_changed()
    }

    /// 更新某台主机的远程命令输入草稿。
    pub(super) fn update_host_command_draft(
        &mut self,
        host_id: HostId,
        command: String,
    ) -> AppUpdateOutcome {
        self.ui.set_remote_command(host_id, command);
        draft_changed()
    }

    /// 更新某台主机的 SFTP 初始路径输入草稿。
    pub(super) fn update_host_sftp_initial_dir_draft(
        &mut self,
        host_id: HostId,
        initial_dir: String,
    ) -> AppUpdateOutcome {
        self.ui.set_sftp_initial_dir(host_id, initial_dir);
        draft_changed()
    }
}

fn draft_changed() -> AppUpdateOutcome {
    AppUpdateOutcome {
        state_changed: true,
        ..AppUpdateOutcome::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AuthProfile;
    use crate::model::Message;
    use crate::model::QuickHostAuthField;
    use crate::model::QuickHostAuthKind;
    use crate::model::QuickHostDraftField;
    use crate::model::SecretRef;
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
}
