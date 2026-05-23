//! SFTP 面板展示模型类型。

/// SFTP 文件列表展示行。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct SftpEntryViewModel {
    pub name: String,
    pub path: String,
    pub kind: &'static str,
    pub size: String,
    pub selected: bool,
}

/// 当前 SFTP 区域展示状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::app) struct SftpViewModel {
    pub host_id: String,
    pub title: String,
    pub current_dir: String,
    pub selected_path: String,
    pub loading: bool,
    pub last_error: String,
    pub entries: Vec<SftpEntryViewModel>,
}
