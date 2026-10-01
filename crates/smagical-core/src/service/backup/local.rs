//! 本地文件系统容灾备份驱动 (Local Filesystem Driver)

use std::path::PathBuf;
use async_trait::async_trait;

use super::{compute_sha256, BackupDriver, RemoteSnapshotInfo};

/// 本地目录存储驱动
pub struct LocalBackupDriver {
    base_dir: PathBuf,
}

impl LocalBackupDriver {
    /// 构造本地存储驱动
    pub fn new(endpoint: &str) -> Self {
        let path = if endpoint.trim().is_empty() {
            // 默认回退至当前用户主目录下的 .smalux/backups
            directories::BaseDirs::new()
                .map(|b| b.home_dir().join(".smalux").join("backups"))
                .unwrap_or_else(|| PathBuf::from("backups"))
        } else {
            PathBuf::from(endpoint.trim())
        };
        Self { base_dir: path }
    }

    fn ensure_dir(&self) -> Result<(), String> {
        if !self.base_dir.exists() {
            std::fs::create_dir_all(&self.base_dir)
                .map_err(|e| format!("无法创建本地备份目录 {}: {}", self.base_dir.display(), e))?;
        }
        Ok(())
    }
}

#[async_trait]
impl BackupDriver for LocalBackupDriver {
    async fn test_connection(&self) -> Result<(), String> {
        let dir = self.base_dir.clone();
        tokio::task::spawn_blocking(move || {
            if !dir.exists() {
                std::fs::create_dir_all(&dir)
                    .map_err(|e| format!("无法创建目录 {}: {}", dir.display(), e))?;
            }
            // 写入探针文件测试写入权限
            let probe_file = dir.join(".probe_write_test");
            std::fs::write(&probe_file, b"ok")
                .map_err(|e| format!("本地备份目录无写入权限 {}: {}", dir.display(), e))?;
            let _ = std::fs::remove_file(probe_file);
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    async fn push_snapshot(&self, snapshot_id: &str, payload: &[u8]) -> Result<String, String> {
        self.ensure_dir()?;
        let file_name = format!("snapshot_{}.json", snapshot_id);
        let target_path = self.base_dir.join(&file_name);
        let data = payload.to_vec();

        tokio::task::spawn_blocking(move || {
            std::fs::write(&target_path, data)
                .map_err(|e| format!("写入快照文件失败 {}: {}", target_path.display(), e))?;
            Ok::<_, String>(file_name)
        })
        .await
        .map_err(|e| e.to_string())?
    }

    async fn list_snapshots(&self) -> Result<Vec<RemoteSnapshotInfo>, String> {
        let dir = self.base_dir.clone();
        tokio::task::spawn_blocking(move || {
            if !dir.exists() {
                return Ok(Vec::new());
            }

            let mut list = Vec::new();
            let entries = std::fs::read_dir(&dir)
                .map_err(|e| format!("读取本地备份目录失败: {}", e))?;

            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
                    if name.starts_with("snapshot_") && name.ends_with(".json") {
                        if let Ok(meta) = entry.metadata() {
                            let size_bytes = meta.len();
                            let epoch_secs = meta
                                .modified()
                                .ok()
                                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                .map(|d| d.as_secs())
                                .unwrap_or_default();

                            let timestamp = chrono::DateTime::from_timestamp(epoch_secs as i64, 0)
                                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                                .unwrap_or_else(|| "未知时间".to_string());

                            // 读取前几个字节或计算快速校验值
                            let hash = if let Ok(bytes) = std::fs::read(&path) {
                                compute_sha256(&bytes)
                            } else {
                                "sha256:unknown".to_string()
                            };

                            list.push(RemoteSnapshotInfo {
                                remote_id: name.to_string(),
                                timestamp,
                                epoch_secs,
                                size_bytes,
                                hash,
                            });
                        }
                    }
                }
            }

            list.sort_by(|a, b| b.epoch_secs.cmp(&a.epoch_secs));
            Ok(list)
        })
        .await
        .map_err(|e| e.to_string())?
    }

    async fn pull_snapshot(&self, remote_id: &str) -> Result<Vec<u8>, String> {
        let target_path = self.base_dir.join(remote_id);
        tokio::task::spawn_blocking(move || {
            std::fs::read(&target_path)
                .map_err(|e| format!("读取本地快照失败 {}: {}", target_path.display(), e))
        })
        .await
        .map_err(|e| e.to_string())?
    }

    async fn delete_snapshot(&self, remote_id: &str) -> Result<(), String> {
        let target_path = self.base_dir.join(remote_id);
        tokio::task::spawn_blocking(move || {
            if target_path.exists() {
                std::fs::remove_file(&target_path)
                    .map_err(|e| format!("删除快照文件失败 {}: {}", target_path.display(), e))?;
            }
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
    }
}
