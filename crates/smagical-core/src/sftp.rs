//! SFTP 浏览和传输状态模型。

use serde::{Deserialize, Serialize};

use crate::{HostId, SessionId, TransferId};

/// SFTP 书签。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SftpBookmark {
    pub host_id: HostId,
    pub label: String,
    pub remote_path: String,
}

/// SFTP 目录项类型。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SftpEntryKind {
    File,
    Directory,
    Symlink,
    Other,
}

/// SFTP 目录项。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SftpEntry {
    pub name: String,
    pub remote_path: String,
    pub kind: SftpEntryKind,
    pub size: Option<u64>,
    pub modified_at_unix_secs: Option<u64>,
    pub permissions: Option<u32>,
}

impl SftpEntry {
    /// 判断目录项是否可以进入。
    pub fn is_navigable(&self) -> bool {
        matches!(self.kind, SftpEntryKind::Directory | SftpEntryKind::Symlink)
    }
}

/// SFTP 浏览面板运行态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SftpBrowserState {
    /// 当前拥有该浏览器状态的 SFTP 会话，用于丢弃旧会话迟到事件。
    pub session_id: SessionId,
    pub host_id: HostId,
    pub current_dir: String,
    pub entries: Vec<SftpEntry>,
    pub selected_path: Option<String>,
    pub loading: bool,
    pub last_error: Option<String>,
}

/// SFTP 传输方向。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferDirection {
    Upload,
    Download,
}

/// SFTP 传输状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferStatus {
    Queued,
    Running,
    Completed,
    Failed { reason: String },
    Cancelled,
}

impl TransferStatus {
    /// 判断传输任务是否已经进入终态，终态不再接受迟到进度更新。
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TransferStatus::Completed | TransferStatus::Failed { .. } | TransferStatus::Cancelled
        )
    }

    /// 判断传输任务是否仍在本地队列中，只有队列态可以被本地取消。
    pub fn is_queued(&self) -> bool {
        matches!(self, TransferStatus::Queued)
    }
}

/// SFTP 上传或下载任务。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferTask {
    pub id: TransferId,
    pub session_id: SessionId,
    pub host_id: HostId,
    pub direction: TransferDirection,
    pub local_path: String,
    pub remote_path: String,
    pub total_bytes: Option<u64>,
    pub transferred_bytes: u64,
    pub status: TransferStatus,
}

impl TransferTask {
    /// 返回传输进度，范围为 `0.0..=1.0`。
    pub fn progress(&self) -> f32 {
        match self.total_bytes {
            Some(0) => 1.0,
            Some(total) => (self.transferred_bytes as f32 / total as f32).clamp(0.0, 1.0),
            None => 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn sftp_entry_reports_navigable_kinds() {
        let directory = SftpEntry {
            name: "logs".to_owned(),
            remote_path: "/var/log".to_owned(),
            kind: SftpEntryKind::Directory,
            size: None,
            modified_at_unix_secs: None,
            permissions: Some(0o755),
        };
        let file = SftpEntry {
            name: "syslog".to_owned(),
            remote_path: "/var/log/syslog".to_owned(),
            kind: SftpEntryKind::File,
            size: Some(1024),
            modified_at_unix_secs: Some(1_700_000_000),
            permissions: Some(0o644),
        };

        assert!(directory.is_navigable());
        assert!(!file.is_navigable());
    }

    #[test]
    fn transfer_task_progress_is_clamped() {
        let task = TransferTask {
            id: TransferId(Uuid::new_v4()),
            session_id: SessionId(Uuid::new_v4()),
            host_id: HostId(Uuid::new_v4()),
            direction: TransferDirection::Download,
            local_path: "C:/tmp/syslog".to_owned(),
            remote_path: "/var/log/syslog".to_owned(),
            total_bytes: Some(100),
            transferred_bytes: 150,
            status: TransferStatus::Running,
        };

        assert_eq!(task.progress(), 1.0);
    }

    #[test]
    fn transfer_status_lifecycle_helpers_are_centralized() {
        assert!(!TransferStatus::Queued.is_terminal());
        assert!(!TransferStatus::Running.is_terminal());
        assert!(TransferStatus::Completed.is_terminal());
        assert!(
            TransferStatus::Failed {
                reason: "network".to_owned()
            }
            .is_terminal()
        );
        assert!(TransferStatus::Cancelled.is_terminal());

        assert!(TransferStatus::Queued.is_queued());
        assert!(!TransferStatus::Running.is_queued());
        assert!(!TransferStatus::Completed.is_queued());
    }

    #[test]
    fn sftp_state_round_trips_through_toml() {
        let host_id = HostId(Uuid::new_v4());
        let state = SftpBrowserState {
            session_id: SessionId(Uuid::new_v4()),
            host_id,
            current_dir: "/home/ops".to_owned(),
            entries: vec![SftpEntry {
                name: "deploy.sh".to_owned(),
                remote_path: "/home/ops/deploy.sh".to_owned(),
                kind: SftpEntryKind::File,
                size: Some(2048),
                modified_at_unix_secs: Some(1_700_000_000),
                permissions: Some(0o755),
            }],
            selected_path: Some("/home/ops/deploy.sh".to_owned()),
            loading: false,
            last_error: None,
        };

        let encoded = toml::to_string(&state).expect("SFTP 浏览态应该可以序列化为 TOML");
        let decoded: SftpBrowserState =
            toml::from_str(&encoded).expect("SFTP 浏览态应该可以从 TOML 反序列化");

        assert_eq!(decoded, state);
    }

    #[test]
    fn transfer_task_round_trips_through_toml() {
        let task = TransferTask {
            id: TransferId(Uuid::new_v4()),
            session_id: SessionId(Uuid::new_v4()),
            host_id: HostId(Uuid::new_v4()),
            direction: TransferDirection::Upload,
            local_path: "C:/deploy/app.tar.gz".to_owned(),
            remote_path: "/tmp/app.tar.gz".to_owned(),
            total_bytes: Some(4096),
            transferred_bytes: 1024,
            status: TransferStatus::Queued,
        };

        let encoded = toml::to_string(&task).expect("SFTP 传输任务应该可以序列化为 TOML");
        let decoded: TransferTask =
            toml::from_str(&encoded).expect("SFTP 传输任务应该可以从 TOML 反序列化");

        assert_eq!(decoded, task);
    }
}
