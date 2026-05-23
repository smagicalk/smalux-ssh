//! SFTP 操作草稿 UI 状态访问。

use super::super::{HostId, UiState};
use super::{SftpActionDraft, SftpActionDraftField};

impl UiState {
    /// 返回指定主机的 SFTP 本地路径草稿；没有草稿时返回空字符串。
    pub fn sftp_local_path_for(&self, host_id: HostId) -> &str {
        self.sftp_action_drafts
            .iter()
            .find(|draft| draft.host_id == host_id)
            .map(|draft| draft.local_path.as_str())
            .unwrap_or("")
    }

    /// 返回指定主机的 SFTP 远程文件名草稿；没有草稿时返回空字符串。
    pub fn sftp_remote_name_for(&self, host_id: HostId) -> &str {
        self.sftp_action_drafts
            .iter()
            .find(|draft| draft.host_id == host_id)
            .map(|draft| draft.remote_name.as_str())
            .unwrap_or("")
    }

    /// 返回指定主机的新目录名草稿；没有草稿时返回空字符串。
    pub fn sftp_new_dir_name_for(&self, host_id: HostId) -> &str {
        self.sftp_action_drafts
            .iter()
            .find(|draft| draft.host_id == host_id)
            .map(|draft| draft.new_dir_name.as_str())
            .unwrap_or("")
    }

    /// 更新指定主机的 SFTP 操作草稿。
    pub fn set_sftp_action_field(
        &mut self,
        host_id: HostId,
        field: SftpActionDraftField,
        value: impl Into<String>,
    ) {
        let value = value.into();

        match field {
            SftpActionDraftField::LocalPath => {
                self.ensure_sftp_action_draft(host_id).local_path = value
            }
            SftpActionDraftField::RemoteName => {
                self.ensure_sftp_action_draft(host_id).remote_name = value
            }
            SftpActionDraftField::NewDirName => {
                self.ensure_sftp_action_draft(host_id).new_dir_name = value
            }
        }
    }

    fn ensure_sftp_action_draft(&mut self, host_id: HostId) -> &mut SftpActionDraft {
        if let Some(index) = self
            .sftp_action_drafts
            .iter()
            .position(|draft| draft.host_id == host_id)
        {
            return &mut self.sftp_action_drafts[index];
        }

        self.sftp_action_drafts.push(SftpActionDraft::new(host_id));
        self.sftp_action_drafts
            .last_mut()
            .expect("刚插入的 SFTP 操作草稿应该存在")
    }
}
