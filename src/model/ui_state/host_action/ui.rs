//! 主机操作区输入草稿 UI 状态访问。

use super::{DEFAULT_REMOTE_COMMAND, DEFAULT_SFTP_INITIAL_DIR, HostActionDraft};
use crate::model::{HostId, UiState};

impl UiState {
    /// 返回指定主机的远程命令草稿；没有草稿时使用默认命令。
    pub fn remote_command_for(&self, host_id: HostId) -> &str {
        self.host_action_drafts
            .iter()
            .find(|draft| draft.host_id == host_id)
            .map(|draft| draft.remote_command.as_str())
            .unwrap_or(DEFAULT_REMOTE_COMMAND)
    }

    /// 返回指定主机的 SFTP 初始路径草稿；没有草稿时使用根目录。
    pub fn sftp_initial_dir_for(&self, host_id: HostId) -> &str {
        self.host_action_drafts
            .iter()
            .find(|draft| draft.host_id == host_id)
            .map(|draft| draft.sftp_initial_dir.as_str())
            .unwrap_or(DEFAULT_SFTP_INITIAL_DIR)
    }

    /// 更新远程命令输入草稿。
    pub fn set_remote_command(&mut self, host_id: HostId, command: impl Into<String>) {
        self.ensure_host_action_draft(host_id).remote_command = command.into();
    }

    /// 更新 SFTP 初始路径输入草稿。
    pub fn set_sftp_initial_dir(&mut self, host_id: HostId, initial_dir: impl Into<String>) {
        self.ensure_host_action_draft(host_id).sftp_initial_dir = initial_dir.into();
    }

    fn ensure_host_action_draft(&mut self, host_id: HostId) -> &mut HostActionDraft {
        if let Some(index) = self
            .host_action_drafts
            .iter()
            .position(|draft| draft.host_id == host_id)
        {
            return &mut self.host_action_drafts[index];
        }

        self.host_action_drafts.push(HostActionDraft::new(host_id));
        self.host_action_drafts
            .last_mut()
            .expect("刚插入的主机操作草稿应该存在")
    }
}
