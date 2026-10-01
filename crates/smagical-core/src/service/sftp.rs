//! SFTP 远程文件传输与目录管理契约。

use crate::domain::FileItemData;
use crate::service::error::SshServiceResult;
use async_trait::async_trait;
use tokio::sync::mpsc::UnboundedSender;

/// 文件传输进度更新数据载荷
#[derive(Debug, Clone)]
pub struct TransferProgress {
    /// 任务唯一标识 ID
    pub task_id: String,
    /// 已传输字节数
    pub transferred_bytes: u64,
    /// 文件总字节数（若未知则为 0）
    pub total_bytes: u64,
    /// 瞬时传输速率 (字节/秒)
    pub speed_bytes_per_sec: u64,
}

impl TransferProgress {
    /// 计算传输进度百分比 (0.0 ~ 100.0)
    pub fn percentage(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.transferred_bytes as f64 / self.total_bytes as f64 * 100.0).min(100.0)
        }
    }

    /// 计算预估剩余传输时间 (秒)，若速率为 0 或已传输完毕则返回 None
    pub fn eta_seconds(&self) -> Option<u64> {
        if self.speed_bytes_per_sec == 0 || self.total_bytes <= self.transferred_bytes {
            None
        } else {
            let remaining = self.total_bytes - self.transferred_bytes;
            Some(remaining / self.speed_bytes_per_sec)
        }
    }
}

/// SFTP 远程文件传输与文件系统抽象契约
#[async_trait]
pub trait SftpService: Send + Sync {
    /// 列出远程目录下的全部文件与子目录项
    async fn list_dir(&self, session_id: &str, remote_path: &str) -> SshServiceResult<Vec<FileItemData>>;

    /// 查询远程单个路径的元数据属性
    async fn stat_path(&self, session_id: &str, remote_path: &str) -> SshServiceResult<FileItemData>;

    /// 递归创建远程目录
    async fn create_dir(&self, session_id: &str, remote_path: &str) -> SshServiceResult<()>;

    /// 删除远程文件或目录 (recursive: 是否递归级联删除)
    async fn remove_path(&self, session_id: &str, remote_path: &str, recursive: bool) -> SshServiceResult<()>;

    /// 重命名或移动远程文件/目录
    async fn rename(&self, session_id: &str, old_path: &str, new_path: &str) -> SshServiceResult<()>;

    /// 流式下载远程文件至本地物理路径
    async fn download_file(
        &self,
        session_id: &str,
        remote_path: &str,
        local_path: &str,
        progress_tx: Option<UnboundedSender<TransferProgress>>,
    ) -> SshServiceResult<()>;

    /// 流式上传本地物理文件至远程路径
    async fn upload_file(
        &self,
        session_id: &str,
        local_path: &str,
        remote_path: &str,
        progress_tx: Option<UnboundedSender<TransferProgress>>,
    ) -> SshServiceResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_progress_percentage_and_eta() {
        let progress = TransferProgress {
            task_id: "test-task".to_string(),
            transferred_bytes: 500,
            total_bytes: 1000,
            speed_bytes_per_sec: 100,
        };

        assert!((progress.percentage() - 50.0).abs() < f64::EPSILON);
        assert_eq!(progress.eta_seconds(), Some(5));

        let completed = TransferProgress {
            task_id: "test-done".to_string(),
            transferred_bytes: 1000,
            total_bytes: 1000,
            speed_bytes_per_sec: 200,
        };
        assert!((completed.percentage() - 100.0).abs() < f64::EPSILON);
        assert_eq!(completed.eta_seconds(), None);

        let zero_speed = TransferProgress {
            task_id: "test-zero".to_string(),
            transferred_bytes: 100,
            total_bytes: 1000,
            speed_bytes_per_sec: 0,
        };
        assert_eq!(zero_speed.eta_seconds(), None);
    }
}

