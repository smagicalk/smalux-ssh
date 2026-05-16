//! SFTP 操作草稿。

use super::HostId;
use super::UiState;

/// 按主机保存的 SFTP 操作草稿。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SftpActionDraft {
    pub host_id: HostId,
    pub local_path: String,
    pub remote_name: String,
    pub new_dir_name: String,
}

impl SftpActionDraft {
    /// 为主机创建空的 SFTP 操作草稿。
    pub fn new(host_id: HostId) -> Self {
        Self {
            host_id,
            local_path: String::new(),
            remote_name: String::new(),
            new_dir_name: String::new(),
        }
    }
}

/// SFTP 操作草稿字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SftpActionDraftField {
    LocalPath,
    RemoteName,
    NewDirName,
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn sftp_action_draft_starts_empty() {
        let draft = SftpActionDraft::new(HostId(Uuid::new_v4()));

        assert!(draft.local_path.is_empty());
        assert!(draft.remote_name.is_empty());
        assert!(draft.new_dir_name.is_empty());
    }

    #[test]
    fn ui_state_sftp_action_messages_update_form_only() {
        let mut state = UiState::default();
        let host_id = HostId(Uuid::new_v4());

        state.set_sftp_action_field(
            host_id,
            SftpActionDraftField::LocalPath,
            "C:/tmp/app.tar.gz",
        );
        state.set_sftp_action_field(host_id, SftpActionDraftField::RemoteName, "app.tar.gz");
        state.set_sftp_action_field(host_id, SftpActionDraftField::NewDirName, "releases");

        assert_eq!(state.sftp_local_path_for(host_id), "C:/tmp/app.tar.gz");
        assert_eq!(state.sftp_remote_name_for(host_id), "app.tar.gz");
        assert_eq!(state.sftp_new_dir_name_for(host_id), "releases");
    }
}
