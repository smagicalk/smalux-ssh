//! SFTP 操作草稿。

use super::HostId;

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
}
