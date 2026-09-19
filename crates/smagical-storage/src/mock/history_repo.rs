//! 内存会话历史与屏幕快照仓储实现

use std::sync::{Arc, RwLock};
use smagical_core::domain::history::HistoryRecord;
use smagical_core::storage::{HistoryRepository, StorageError, StorageResult};

/// 线程安全的内存会话历史仓储实现
#[derive(Debug, Default, Clone)]
pub struct MockHistoryRepository {
    history: Arc<RwLock<Vec<HistoryRecord>>>,
    snapshots: Arc<RwLock<std::collections::HashMap<String, String>>>,
}

impl MockHistoryRepository {
    /// 创建空的内存历史仓储
    pub fn new() -> Self {
        Self {
            history: Arc::new(RwLock::new(Vec::new())),
            snapshots: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// 使用指定历史列表创建内存仓储
    pub fn with_history(history: Vec<HistoryRecord>) -> Self {
        Self {
            history: Arc::new(RwLock::new(history)),
            snapshots: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// 使用指定历史列表与快照集合创建内存仓储
    pub fn with_history_and_snapshots(
        history: Vec<HistoryRecord>,
        snapshots: std::collections::HashMap<String, String>,
    ) -> Self {
        Self {
            history: Arc::new(RwLock::new(history)),
            snapshots: Arc::new(RwLock::new(snapshots)),
        }
    }
}

#[async_trait::async_trait]
impl HistoryRepository for MockHistoryRepository {
    async fn list_all(&self) -> StorageResult<Vec<HistoryRecord>> {
        let read_guard = self.history.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        let mut list = read_guard.clone();
        // 默认排序：置顶在最前，其余按 connected_at 倒序排列
        list.sort_by(|a, b| {
            b.is_pinned.cmp(&a.is_pinned)
                .then_with(|| b.connected_at.cmp(&a.connected_at))
        });
        Ok(list)
    }

    async fn get_by_id(&self, id: &str) -> StorageResult<Option<HistoryRecord>> {
        let read_guard = self.history.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(read_guard.iter().find(|h| h.id == id).cloned())
    }

    async fn save(&self, record: &HistoryRecord) -> StorageResult<()> {
        let snapshot_to_delete = {
            let mut write_guard = self.history.write().map_err(|e| StorageError::Backend(e.to_string()))?;
            if let Some(pos) = write_guard.iter().position(|h| h.id == record.id) {
                write_guard[pos] = record.clone();
            } else {
                write_guard.push(record.clone());
            }
            // 限制最大 500 条容量上限（超量时淘汰最早的非置顶记录）
            if write_guard.len() > 500
                && let Some(oldest_idx) = write_guard
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| !r.is_pinned)
                    .min_by_key(|(_, r)| r.connected_at)
                    .map(|(idx, _)| idx)
            {
                let removed = write_guard.remove(oldest_idx);
                Some(removed.id)
            } else {
                None
            }
        };

        if let Some(ref rem_id) = snapshot_to_delete {
            let _ = self.delete_snapshot(rem_id).await;
        }

        tracing::debug!(target: "smagical_storage::mock", "MockStorage 保存历史记录: {} ({})", record.title, record.address);
        Ok(())
    }

    async fn save_batch(&self, records: &[HistoryRecord]) -> StorageResult<()> {
        let mut write_guard = self.history.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        for record in records {
            if let Some(pos) = write_guard.iter().position(|h| h.id == record.id) {
                write_guard[pos] = record.clone();
            } else {
                write_guard.push(record.clone());
            }
        }
        tracing::debug!(target: "smagical_storage::mock", "MockStorage 批量保存历史记录: {} 条", records.len());
        Ok(())
    }

    async fn delete(&self, id: &str) -> StorageResult<bool> {
        let removed = {
            let mut write_guard = self.history.write().map_err(|e| StorageError::Backend(e.to_string()))?;
            if let Some(pos) = write_guard.iter().position(|h| h.id == id) {
                Some(write_guard.remove(pos))
            } else {
                None
            }
        };

        if let Some(rem) = removed {
            let _ = self.delete_snapshot(id).await;
            tracing::info!(target: "smagical_storage::mock", "MockStorage 删除历史记录: {} ({})", rem.title, id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn clear_all(&self, keep_pinned: bool) -> StorageResult<()> {
        let mut write_guard = self.history.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if keep_pinned {
            let unpinned_ids: Vec<String> = write_guard.iter().filter(|h| !h.is_pinned).map(|h| h.id.clone()).collect();
            write_guard.retain(|h| h.is_pinned);
            if let Ok(mut snap_guard) = self.snapshots.write() {
                for uid in unpinned_ids {
                    snap_guard.remove(&uid);
                }
            }
        } else {
            write_guard.clear();
            if let Ok(mut snap_guard) = self.snapshots.write() {
                snap_guard.clear();
            }
        }
        tracing::info!(target: "smagical_storage::mock", "MockStorage 清空历史记录 (keep_pinned: {})", keep_pinned);
        Ok(())
    }

    async fn toggle_pin(&self, id: &str) -> StorageResult<bool> {
        let mut write_guard = self.history.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(h) = write_guard.iter_mut().find(|h| h.id == id) {
            h.is_pinned = !h.is_pinned;
            tracing::info!(target: "smagical_storage::mock", "MockStorage 切换历史记录置顶状态: {} -> {}", id, h.is_pinned);
            Ok(h.is_pinned)
        } else {
            Err(StorageError::NotFound(format!("历史记录 ID 未找到: {}", id)))
        }
    }

    async fn save_snapshot(&self, history_id: &str, content: &str, max_lines: usize) -> StorageResult<()> {
        let truncated_content = if max_lines > 0 {
            let lines: Vec<&str> = content.lines().collect();
            if lines.len() > max_lines {
                lines[lines.len() - max_lines..].join("\n")
            } else {
                content.to_string()
            }
        } else {
            content.to_string()
        };

        let mut write_guard = self.snapshots.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        write_guard.insert(history_id.to_string(), truncated_content);
        tracing::debug!(target: "smagical_storage::mock", "MockStorage 保存终端屏幕快照: {} (max_lines: {})", history_id, max_lines);
        Ok(())
    }

    async fn get_snapshot(&self, history_id: &str) -> StorageResult<Option<String>> {
        let read_guard = self.snapshots.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(read_guard.get(history_id).cloned())
    }

    async fn delete_snapshot(&self, history_id: &str) -> StorageResult<bool> {
        let mut write_guard = self.snapshots.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(write_guard.remove(history_id).is_some())
    }
}
