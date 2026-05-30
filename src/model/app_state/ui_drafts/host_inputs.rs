//! 主机作用域输入草稿处理。
//!
//! 这些草稿都挂在具体主机下，用于“打开终端前输入远程命令”或“SFTP 打开前填写路径”。
//! 它们不是持久化配置，用户离开当前操作后可以安全重置。

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
        // 只更新 UI 草稿，不立即创建命令历史；真正执行时再记录历史。
        self.ui.set_remote_command(host_id, command);
        draft_changed()
    }

    /// 更新某台主机的 SFTP 初始路径输入草稿。
    pub(in crate::model::app_state) fn update_host_sftp_initial_dir_draft(
        &mut self,
        host_id: HostId,
        initial_dir: String,
    ) -> AppUpdateOutcome {
        // 初始目录由启动 SFTP 时读取，允许用户在连接前反复编辑。
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
        // SFTP 操作弹窗按字段更新，避免 UI 层理解草稿内部结构。
        self.ui.set_sftp_action_field(host_id, field, value);
        draft_changed()
    }
}
