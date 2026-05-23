//! 主机操作区输入草稿类型。

use crate::model::HostId;

pub const DEFAULT_REMOTE_COMMAND: &str = "uptime";
pub const DEFAULT_SFTP_INITIAL_DIR: &str = "/";

/// 每台主机在操作区的输入草稿。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostActionDraft {
    pub host_id: HostId,
    pub remote_command: String,
    pub sftp_initial_dir: String,
}

impl HostActionDraft {
    /// 为主机创建可直接使用的默认操作草稿。
    pub fn new(host_id: HostId) -> Self {
        Self {
            host_id,
            remote_command: DEFAULT_REMOTE_COMMAND.to_owned(),
            sftp_initial_dir: DEFAULT_SFTP_INITIAL_DIR.to_owned(),
        }
    }
}
