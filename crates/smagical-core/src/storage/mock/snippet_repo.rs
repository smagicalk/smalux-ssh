//! 内存代码片段与多层分组仓储实现

use std::sync::{Arc, RwLock};
use crate::domain::snippet::{SnippetGroupRecord, SnippetRecord};
use crate::storage::{SnippetRepository, StorageError, StorageResult};

/// 线程安全的内存代码片段仓储实现
#[derive(Debug, Default, Clone)]
pub struct MockSnippetRepository {
    snippets: Arc<RwLock<Vec<SnippetRecord>>>,
    groups: Arc<RwLock<Vec<SnippetGroupRecord>>>,
}

impl MockSnippetRepository {
    /// 创建空的内存代码片段仓储
    pub fn new() -> Self {
        Self {
            snippets: Arc::new(RwLock::new(Vec::new())),
            groups: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 使用指定片段与分组列表创建内存仓储
    pub fn with_data(snippets: Vec<SnippetRecord>, groups: Vec<SnippetGroupRecord>) -> Self {
        Self {
            snippets: Arc::new(RwLock::new(snippets)),
            groups: Arc::new(RwLock::new(groups)),
        }
    }
}

impl SnippetRepository for MockSnippetRepository {
    fn list_all(&self) -> StorageResult<Vec<SnippetRecord>> {
        let read_guard = self.snippets.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(read_guard.clone())
    }

    fn list_by_group(&self, group_id: Option<&str>) -> StorageResult<Vec<SnippetRecord>> {
        let all = self.list_all()?;
        Ok(all.into_iter().filter(|s| s.parent_group_id.as_deref() == group_id).collect())
    }

    fn get_by_id(&self, id: &str) -> StorageResult<Option<SnippetRecord>> {
        let read_guard = self.snippets.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(read_guard.iter().find(|s| s.id == id).cloned())
    }

    fn search(&self, query: &str) -> StorageResult<Vec<SnippetRecord>> {
        let all = self.list_all()?;
        if query.trim().is_empty() {
            return Ok(all);
        }
        let q = query.to_lowercase();
        Ok(all.into_iter().filter(|s| {
            s.title.to_lowercase().contains(&q)
                || s.content.to_lowercase().contains(&q)
                || s.description.to_lowercase().contains(&q)
                || s.tags.iter().any(|t| t.to_lowercase().contains(&q))
                || s.language.to_lowercase().contains(&q)
        }).collect())
    }

    fn save(&self, record: &SnippetRecord) -> StorageResult<()> {
        let mut write_guard = self.snippets.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(pos) = write_guard.iter().position(|s| s.id == record.id) {
            write_guard[pos] = record.clone();
        } else {
            write_guard.push(record.clone());
        }
        tracing::debug!(target: "smagical_core::storage", "MockStorage 保存代码片段: {} ({})", record.title, record.language);
        Ok(())
    }

    fn save_batch(&self, records: &[SnippetRecord]) -> StorageResult<()> {
        let mut write_guard = self.snippets.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        for rec in records {
            if let Some(pos) = write_guard.iter().position(|s| s.id == rec.id) {
                write_guard[pos] = rec.clone();
            } else {
                write_guard.push(rec.clone());
            }
        }
        Ok(())
    }

    fn delete(&self, id: &str) -> StorageResult<bool> {
        let mut write_guard = self.snippets.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(pos) = write_guard.iter().position(|s| s.id == id) {
            write_guard.remove(pos);
            tracing::debug!(target: "smagical_core::storage", "MockStorage 删除代码片段: ID={}", id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn toggle_favorite(&self, id: &str) -> StorageResult<bool> {
        let mut write_guard = self.snippets.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(snip) = write_guard.iter_mut().find(|s| s.id == id) {
            snip.is_favorite = !snip.is_favorite;
            Ok(snip.is_favorite)
        } else {
            Ok(false)
        }
    }

    fn list_groups(&self) -> StorageResult<Vec<SnippetGroupRecord>> {
        let read_guard = self.groups.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(read_guard.clone())
    }

    fn get_group_by_id(&self, id: &str) -> StorageResult<Option<SnippetGroupRecord>> {
        let read_guard = self.groups.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(read_guard.iter().find(|g| g.id == id).cloned())
    }

    fn save_group(&self, group: &SnippetGroupRecord) -> StorageResult<()> {
        let mut write_guard = self.groups.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(pos) = write_guard.iter().position(|g| g.id == group.id) {
            write_guard[pos] = group.clone();
        } else {
            write_guard.push(group.clone());
        }
        Ok(())
    }

    fn delete_group(&self, id: &str) -> StorageResult<bool> {
        let mut g_write = self.groups.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(pos) = g_write.iter().position(|g| g.id == id) {
            let removed = g_write.remove(pos);
            for child in g_write.iter_mut().filter(|g| g.parent_id.as_deref() == Some(id)) {
                child.parent_id = removed.parent_id.clone();
                child.level = child.level.saturating_sub(1);
            }
            if let Ok(mut s_write) = self.snippets.write() {
                for snip in s_write.iter_mut().filter(|s| s.parent_group_id.as_deref() == Some(id)) {
                    snip.parent_group_id = removed.parent_id.clone();
                }
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn set_group_expanded(&self, id: &str, expanded: bool) -> StorageResult<()> {
        let mut write_guard = self.groups.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(g) = write_guard.iter_mut().find(|g| g.id == id) {
            g.is_expanded = expanded;
        }
        Ok(())
    }

    fn move_group(&self, id: &str, new_parent_id: Option<&str>) -> StorageResult<()> {
        let mut write_guard = self.groups.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        let target_level = if let Some(p_id) = new_parent_id {
            write_guard.iter().find(|g| g.id == p_id).map(|g| g.level + 1).unwrap_or(0)
        } else {
            0
        };
        if let Some(g) = write_guard.iter_mut().find(|g| g.id == id) {
            g.parent_id = new_parent_id.map(|s| s.to_string());
            g.level = target_level;
        }
        Ok(())
    }
}
