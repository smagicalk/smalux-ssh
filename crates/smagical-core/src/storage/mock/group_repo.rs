//! 内存分组仓储实现

use std::sync::{Arc, RwLock};
use crate::domain::group::GroupRecord;
use crate::storage::{GroupRepository, StorageError, StorageResult};

/// 线程安全的内存分组仓储实现
#[derive(Debug, Default, Clone)]
pub struct MockGroupRepository {
    groups: Arc<RwLock<Vec<GroupRecord>>>,
}

impl MockGroupRepository {
    /// 创建空的分组仓储
    pub fn new() -> Self {
        Self {
            groups: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 使用指定分组列表创建内存仓储
    pub fn with_groups(groups: Vec<GroupRecord>) -> Self {
        Self {
            groups: Arc::new(RwLock::new(groups)),
        }
    }
}

impl GroupRepository for MockGroupRepository {
    fn list_all(&self) -> StorageResult<Vec<GroupRecord>> {
        let read_guard = self.groups.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(read_guard.clone())
    }

    fn get_by_id(&self, id: &str) -> StorageResult<Option<GroupRecord>> {
        let read_guard = self.groups.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(read_guard.iter().find(|g| g.id == id).cloned())
    }

    fn save(&self, group: &GroupRecord) -> StorageResult<()> {
        let mut write_guard = self.groups.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(pos) = write_guard.iter().position(|g| g.id == group.id) {
            write_guard[pos] = group.clone();
        } else {
            write_guard.push(group.clone());
        }
        tracing::debug!(target: "smagical_core::storage", "MockStorage 保存分组: {} (ID: {})", group.name, group.id);
        Ok(())
    }

    fn delete(&self, id: &str) -> StorageResult<bool> {
        let mut write_guard = self.groups.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(pos) = write_guard.iter().position(|g| g.id == id) {
            let removed = write_guard.remove(pos);
            tracing::info!(target: "smagical_core::storage", "MockStorage 删除分组: {} (ID: {})", removed.name, id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn set_expanded(&self, id: &str, expanded: bool) -> StorageResult<()> {
        let mut write_guard = self.groups.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(g) = write_guard.iter_mut().find(|g| g.id == id) {
            g.is_expanded = expanded;
            tracing::debug!(target: "smagical_core::storage", "MockStorage 切换分组展开状态: {} -> {}", id, expanded);
            Ok(())
        } else {
            Err(StorageError::NotFound(format!("分组不存在: {}", id)))
        }
    }

    fn move_group(&self, id: &str, new_parent_id: Option<&str>) -> StorageResult<()> {
        let mut write_guard = self.groups.write().map_err(|e| StorageError::Backend(e.to_string()))?;

        let new_parent_str = new_parent_id.map(|s| s.to_string());

        // 计算新层级
        let new_level = match &new_parent_str {
            Some(p_id) => {
                write_guard.iter().find(|g| &g.id == p_id)
                    .ok_or_else(|| StorageError::NotFound(format!("目标父级分组不存在: {}", p_id)))?
                    .level + 1
            }
            None => 0,
        };

        // 获取旧层级
        let old_level = write_guard.iter().find(|g| g.id == id)
            .ok_or_else(|| StorageError::NotFound(format!("分组不存在: {}", id)))?
            .level;

        let level_delta = new_level - old_level;

        // BFS 收集所有后裔分组 ID（先只读，不持有可变引用）
        let mut descendants: Vec<String> = Vec::new();
        let mut frontier = vec![id.to_string()];
        while !frontier.is_empty() {
            let mut next_frontier = Vec::new();
            for parent_id in &frontier {
                for g in write_guard.iter() {
                    if g.parent_id.as_deref() == Some(parent_id.as_str()) {
                        descendants.push(g.id.clone());
                        next_frontier.push(g.id.clone());
                    }
                }
            }
            frontier = next_frontier;
        }

        // 更新目标分组自身
        if let Some(g) = write_guard.iter_mut().find(|g| g.id == id) {
            g.parent_id = new_parent_str.clone();
            g.level = new_level;
        }

        // 递归更新所有后裔分组层级
        if level_delta != 0 {
            for desc_id in &descendants {
                if let Some(g) = write_guard.iter_mut().find(|g| &g.id == desc_id) {
                    g.level += level_delta;
                }
            }
        }

        tracing::info!(
            target: "smagical_core::storage",
            "MockStorage 迁移分组: {} -> 上级: {:?} (递归更新 {} 个后裔分组层级)",
            id, new_parent_str, descendants.len()
        );
        Ok(())
    }
}
