//! UI 输入草稿消息处理。

use crate::model::HostId;

use super::{AppState, AppUpdateOutcome};

impl AppState {
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
    use crate::model::Message;
    use uuid::Uuid;

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
