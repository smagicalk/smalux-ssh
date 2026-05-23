//! SFTP 操作草稿类型。

use super::super::HostId;

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
