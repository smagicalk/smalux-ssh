//! 主机作用域输入草稿处理。

use crate::model::{HostId, SftpActionDraftField};

use super::super::{AppState, AppUpdateOutcome};
use super::draft_changed;

impl AppState {
    /// 更新某台主机的远程命令输入草稿。
    pub(in crate::model::app_state) fn update_host_command_draft(
        &mut self,
        host_id: HostId,
        command: String,
    ) -> AppUpdateOutcome {
        self.ui.set_remote_command(host_id, command);
        draft_changed()
    }

    /// 更新某台主机的 SFTP 初始路径输入草稿。
    pub(in crate::model::app_state) fn update_host_sftp_initial_dir_draft(
        &mut self,
        host_id: HostId,
        initial_dir: String,
    ) -> AppUpdateOutcome {
        self.ui.set_sftp_initial_dir(host_id, initial_dir);
        draft_changed()
    }

    /// 更新 SFTP 操作草稿。
    pub(in crate::model::app_state) fn update_sftp_action_draft(
        &mut self,
        host_id: HostId,
        field: SftpActionDraftField,
        value: String,
    ) -> AppUpdateOutcome {
        self.ui.set_sftp_action_field(host_id, field, value);
        draft_changed()
    }
}
