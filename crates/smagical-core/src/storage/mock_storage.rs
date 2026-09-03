use std::sync::{Arc, RwLock};
use crate::domain::{
    config::AppConfigRecord,
    credential::{CredentialRecord, CredentialType},
    group::GroupRecord,
    history::HistoryRecord,
    host::{HostRecord, HostStatus},
    snippet::{SnippetGroupRecord, SnippetRecord},
    tunnel::{TunnelRecord, TunnelType},
};
use super::{
    AppStorage, ConfigRepository, CredentialRepository, GroupRepository, HistoryRepository, HostRepository,
    SnippetRepository, TunnelRepository, StorageError, StorageResult,
};



/// 线程安全的内存主机仓储实现
#[derive(Debug, Default, Clone)]
pub struct MockHostRepository {
    hosts: Arc<RwLock<Vec<HostRecord>>>,
}

impl MockHostRepository {
    /// 创建空的内存主机仓储
    pub fn new() -> Self {
        Self {
            hosts: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 使用指定主机列表创建内存仓储
    pub fn with_hosts(hosts: Vec<HostRecord>) -> Self {
        Self {
            hosts: Arc::new(RwLock::new(hosts)),
        }
    }

    /// 获取底层并发锁句柄 (供仓储间引用计算)
    pub fn hosts_raw(&self) -> Arc<RwLock<Vec<HostRecord>>> {
        self.hosts.clone()
    }
}

impl HostRepository for MockHostRepository {
    fn list_all(&self) -> StorageResult<Vec<HostRecord>> {
        let read_guard = self.hosts.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(read_guard.clone())
    }

    fn get_by_id(&self, id: &str) -> StorageResult<Option<HostRecord>> {
        let read_guard = self.hosts.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(read_guard.iter().find(|h| h.id == id).cloned())
    }

    fn list_by_credential(&self, credential_id: &str) -> StorageResult<Vec<HostRecord>> {
        let read_guard = self.hosts.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(read_guard.iter().filter(|h| h.credential_id.as_deref() == Some(credential_id)).cloned().collect())
    }

    fn save(&self, host: &HostRecord) -> StorageResult<()> {
        let mut write_guard = self.hosts.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(pos) = write_guard.iter().position(|h| h.id == host.id) {
            write_guard[pos] = host.clone();
        } else {
            write_guard.push(host.clone());
        }
        tracing::debug!(target: "smagical_core::storage", "MockStorage 保存主机: {} ({}:{})", host.name, host.address, host.port);
        Ok(())
    }

    fn save_batch(&self, hosts: &[HostRecord]) -> StorageResult<()> {
        let mut write_guard = self.hosts.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        for host in hosts {
            if let Some(pos) = write_guard.iter().position(|h| h.id == host.id) {
                write_guard[pos] = host.clone();
            } else {
                write_guard.push(host.clone());
            }
        }
        tracing::debug!(target: "smagical_core::storage", "MockStorage 批量保存主机: {} 台", hosts.len());
        Ok(())
    }

    fn delete(&self, id: &str) -> StorageResult<bool> {
        let mut write_guard = self.hosts.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(pos) = write_guard.iter().position(|h| h.id == id) {
            let removed = write_guard.remove(pos);
            tracing::info!(target: "smagical_core::storage", "MockStorage 删除主机: {} ({})", removed.name, id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn update_list_order(&self, ordered_ids: &[String]) -> StorageResult<()> {
        let mut write_guard = self.hosts.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        let mut reordered = Vec::with_capacity(write_guard.len());
        
        // 1. 先按传入的有序 ID 顺序排入
        for id in ordered_ids {
            if let Some(pos) = write_guard.iter().position(|h| &h.id == id) {
                let mut host = write_guard.remove(pos);
                host.sort_order = reordered.len() as i32;
                reordered.push(host);
            }
        }
        // 2. 将未包含在列表中的其余主机追加至末尾
        for mut host in write_guard.drain(..) {
            host.sort_order = reordered.len() as i32;
            reordered.push(host);
        }

        *write_guard = reordered;
        tracing::debug!(target: "smagical_core::storage", "MockStorage 更新主机列表显示顺序 (共 {} 项)", ordered_ids.len());
        Ok(())
    }
}

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

/// 线程安全的内存历史会话仓储实现
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


impl HistoryRepository for MockHistoryRepository {
    fn list_all(&self) -> StorageResult<Vec<HistoryRecord>> {
        let read_guard = self.history.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        let mut list = read_guard.clone();
        // 默认排序：置顶在最前，其余按 connected_at 倒序排列
        list.sort_by(|a, b| {
            b.is_pinned.cmp(&a.is_pinned)
                .then_with(|| b.connected_at.cmp(&a.connected_at))
        });
        Ok(list)
    }

    fn get_by_id(&self, id: &str) -> StorageResult<Option<HistoryRecord>> {
        let read_guard = self.history.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(read_guard.iter().find(|h| h.id == id).cloned())
    }

    fn save(&self, record: &HistoryRecord) -> StorageResult<()> {
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
            let _ = self.delete_snapshot(&removed.id);
        }

        tracing::debug!(target: "smagical_core::storage", "MockStorage 保存历史记录: {} ({})", record.title, record.address);
        Ok(())
    }

    fn save_batch(&self, records: &[HistoryRecord]) -> StorageResult<()> {
        let mut write_guard = self.history.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        for record in records {
            if let Some(pos) = write_guard.iter().position(|h| h.id == record.id) {
                write_guard[pos] = record.clone();
            } else {
                write_guard.push(record.clone());
            }
        }
        tracing::debug!(target: "smagical_core::storage", "MockStorage 批量保存历史记录: {} 条", records.len());
        Ok(())
    }

    fn delete(&self, id: &str) -> StorageResult<bool> {
        let mut write_guard = self.history.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(pos) = write_guard.iter().position(|h| h.id == id) {
            let removed = write_guard.remove(pos);
            let _ = self.delete_snapshot(id);
            tracing::info!(target: "smagical_core::storage", "MockStorage 删除历史记录: {} ({})", removed.title, id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn clear_all(&self, keep_pinned: bool) -> StorageResult<()> {
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
        tracing::info!(target: "smagical_core::storage", "MockStorage 清空历史记录 (keep_pinned: {})", keep_pinned);
        Ok(())
    }

    fn toggle_pin(&self, id: &str) -> StorageResult<bool> {
        let mut write_guard = self.history.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(h) = write_guard.iter_mut().find(|h| h.id == id) {
            h.is_pinned = !h.is_pinned;
            tracing::info!(target: "smagical_core::storage", "MockStorage 切换历史记录置顶状态: {} -> {}", id, h.is_pinned);
            Ok(h.is_pinned)
        } else {
            Err(StorageError::NotFound(format!("历史记录 ID 未找到: {}", id)))
        }
    }

    fn save_snapshot(&self, history_id: &str, content: &str, max_lines: usize) -> StorageResult<()> {
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
        tracing::debug!(target: "smagical_core::storage", "MockStorage 保存终端屏幕快照: {} (max_lines: {})", history_id, max_lines);
        Ok(())
    }

    fn get_snapshot(&self, history_id: &str) -> StorageResult<Option<String>> {
        let read_guard = self.snapshots.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(read_guard.get(history_id).cloned())
    }

    fn delete_snapshot(&self, history_id: &str) -> StorageResult<bool> {
        let mut write_guard = self.snapshots.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(write_guard.remove(history_id).is_some())
    }
}

/// 线程安全的内存凭据仓储实现
#[derive(Debug, Default, Clone)]
pub struct MockCredentialRepository {
    credentials: Arc<RwLock<Vec<CredentialRecord>>>,
    hosts: Option<Arc<RwLock<Vec<HostRecord>>>>,
}

impl MockCredentialRepository {
    /// 创建空的内存凭据仓储
    pub fn new() -> Self {
        Self {
            credentials: Arc::new(RwLock::new(Vec::new())),
            hosts: None,
        }
    }

    /// 使用指定凭据列表创建内存仓储
    pub fn with_credentials(credentials: Vec<CredentialRecord>) -> Self {
        Self {
            credentials: Arc::new(RwLock::new(credentials)),
            hosts: None,
        }
    }

    /// 关联主机引用创建内存仓储
    pub fn with_hosts(hosts: Arc<RwLock<Vec<HostRecord>>>) -> Self {
        Self {
            credentials: Arc::new(RwLock::new(Vec::new())),
            hosts: Some(hosts),
        }
    }

    /// 使用指定凭据列表并关联主机引用创建内存仓储
    pub fn with_credentials_and_hosts(credentials: Vec<CredentialRecord>, hosts: Arc<RwLock<Vec<HostRecord>>>) -> Self {
        Self {
            credentials: Arc::new(RwLock::new(credentials)),
            hosts: Some(hosts),
        }
    }
}

impl CredentialRepository for MockCredentialRepository {
    fn list_all(&self) -> StorageResult<Vec<CredentialRecord>> {
        let read_guard = self.credentials.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        let hosts_guard = self.hosts.as_ref().and_then(|h| h.read().ok());
        let list = read_guard.iter().map(|c| {
            let mut cred = c.clone();
            if let Some(ref h_list) = hosts_guard {
                cred.bound_host_count = h_list.iter().filter(|h| h.credential_id.as_deref() == Some(&cred.id)).count();
            }
            cred
        }).collect();
        Ok(list)
    }

    fn list_by_type(&self, cred_type: CredentialType) -> StorageResult<Vec<CredentialRecord>> {
        let all = self.list_all()?;
        Ok(all.into_iter().filter(|c| c.cred_type == cred_type).collect())
    }

    fn get_by_id(&self, id: &str) -> StorageResult<Option<CredentialRecord>> {
        let read_guard = self.credentials.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(c) = read_guard.iter().find(|c| c.id == id) {
            let mut cred = c.clone();
            if let Some(ref hosts_lock) = self.hosts {
                if let Ok(h_list) = hosts_lock.read() {
                    cred.bound_host_count = h_list.iter().filter(|h| h.credential_id.as_deref() == Some(&cred.id)).count();
                }
            }
            Ok(Some(cred))
        } else {
            Ok(None)
        }
    }

    fn search(&self, query: &str) -> StorageResult<Vec<CredentialRecord>> {
        let all = self.list_all()?;
        if query.trim().is_empty() {
            return Ok(all);
        }
        let q = query.to_lowercase();
        Ok(all.into_iter().filter(|c| {
            c.name.to_lowercase().contains(&q)
                || c.algorithm.to_lowercase().contains(&q)
                || c.username.as_ref().map_or(false, |u| u.to_lowercase().contains(&q))
                || c.fingerprint.as_ref().map_or(false, |f| f.to_lowercase().contains(&q))
                || c.notes.to_lowercase().contains(&q)
        }).collect())
    }

    fn get_bound_hosts(&self, id: &str) -> StorageResult<Vec<String>> {
        if let Some(ref hosts_lock) = self.hosts {
            let h_list = hosts_lock.read().map_err(|e| StorageError::Backend(e.to_string()))?;
            Ok(h_list.iter().filter(|h| h.credential_id.as_deref() == Some(id)).map(|h| h.id.clone()).collect())
        } else {
            Ok(Vec::new())
        }
    }

    fn save(&self, record: &CredentialRecord) -> StorageResult<()> {
        let mut write_guard = self.credentials.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(pos) = write_guard.iter().position(|c| c.id == record.id) {
            write_guard[pos] = record.clone();
        } else {
            write_guard.push(record.clone());
        }
        tracing::debug!(target: "smagical_core::storage", "MockStorage 保存凭据: {} ({})", record.name, record.algorithm);
        Ok(())
    }

    fn save_batch(&self, records: &[CredentialRecord]) -> StorageResult<()> {
        let mut write_guard = self.credentials.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        for rec in records {
            if let Some(pos) = write_guard.iter().position(|c| c.id == rec.id) {
                write_guard[pos] = rec.clone();
            } else {
                write_guard.push(rec.clone());
            }
        }
        Ok(())
    }

    fn delete(&self, id: &str) -> StorageResult<bool> {
        let mut write_guard = self.credentials.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(pos) = write_guard.iter().position(|c| c.id == id) {
            write_guard.remove(pos);
            tracing::debug!(target: "smagical_core::storage", "MockStorage 删除凭据: ID={}", id);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// 线程安全的内存代码片段与多层层级分组仓储实现
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

/// 线程安全的内存网络隧道与代理仓储实现
#[derive(Debug, Default, Clone)]
pub struct MockTunnelRepository {
    tunnels: Arc<RwLock<Vec<TunnelRecord>>>,
}

impl MockTunnelRepository {
    /// 创建空的内存网络隧道仓储
    pub fn new() -> Self {
        Self {
            tunnels: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 使用指定隧道列表创建内存仓储
    pub fn with_tunnels(tunnels: Vec<TunnelRecord>) -> Self {
        Self {
            tunnels: Arc::new(RwLock::new(tunnels)),
        }
    }
}

impl TunnelRepository for MockTunnelRepository {
    fn list_all(&self) -> StorageResult<Vec<TunnelRecord>> {
        let guard = self.tunnels.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(guard.clone())
    }

    fn list_by_type(&self, tunnel_type: TunnelType) -> StorageResult<Vec<TunnelRecord>> {
        let guard = self.tunnels.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(guard.iter().filter(|t| t.tunnel_type == tunnel_type).cloned().collect())
    }

    fn get_by_id(&self, id: &str) -> StorageResult<Option<TunnelRecord>> {
        let guard = self.tunnels.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(guard.iter().find(|t| t.id == id).cloned())
    }

    fn search(&self, query: &str) -> StorageResult<Vec<TunnelRecord>> {
        let q = query.trim().to_lowercase();
        let guard = self.tunnels.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        if q.is_empty() {
            return Ok(guard.clone());
        }
        Ok(guard.iter().filter(|t| {
            t.name.to_lowercase().contains(&q)
                || t.remote_host.to_lowercase().contains(&q)
                || t.local_port.to_string().contains(&q)
                || t.remote_port.to_string().contains(&q)
                || t.ssh_host_name.to_lowercase().contains(&q)
                || t.notes.to_lowercase().contains(&q)
                || t.tunnel_type.as_str().to_lowercase().contains(&q)
        }).cloned().collect())
    }

    fn save(&self, record: &TunnelRecord) -> StorageResult<()> {
        let mut guard = self.tunnels.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(pos) = guard.iter().position(|t| t.id == record.id) {
            guard[pos] = record.clone();
        } else {
            guard.push(record.clone());
        }
        Ok(())
    }

    fn save_batch(&self, records: &[TunnelRecord]) -> StorageResult<()> {
        for r in records {
            self.save(r)?;
        }
        Ok(())
    }

    fn delete(&self, id: &str) -> StorageResult<bool> {
        let mut guard = self.tunnels.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        let len_before = guard.len();
        guard.retain(|t| t.id != id);
        Ok(guard.len() < len_before)
    }

    fn set_running(&self, id: &str, is_running: bool) -> StorageResult<bool> {
        let mut guard = self.tunnels.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(t) = guard.iter_mut().find(|t| t.id == id) {
            t.is_running = is_running;
            if !is_running {
                t.active_connections = 0;
            } else if t.active_connections == 0 {
                t.active_connections = 1;
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn update_metrics(&self, id: &str, active_conn: usize, bytes_in: u64, bytes_out: u64) -> StorageResult<()> {
        let mut guard = self.tunnels.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(t) = guard.iter_mut().find(|t| t.id == id) {
            t.active_connections = active_conn;
            t.total_bytes_in += bytes_in;
            t.total_bytes_out += bytes_out;
        }
        Ok(())
    }
}

/// 线程安全的内存配置仓储实现
#[derive(Debug, Default, Clone)]
pub struct MockConfigRepository {
    config: Arc<RwLock<AppConfigRecord>>,
}

impl MockConfigRepository {
    /// 创建带有默认应用偏好配置的内存配置仓储
    pub fn new() -> Self {
        Self {
            config: Arc::new(RwLock::new(AppConfigRecord::default())),
        }
    }

    /// 使用自定义配置记录创建内存配置仓储
    pub fn with_config(config: AppConfigRecord) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }
}

impl ConfigRepository for MockConfigRepository {
    fn get(&self) -> StorageResult<AppConfigRecord> {
        let guard = self.config.read().map_err(|e| StorageError::Backend(e.to_string()))?;
        Ok(guard.clone())
    }

    fn save(&self, config: &AppConfigRecord) -> StorageResult<()> {
        let mut guard = self.config.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        *guard = config.clone();
        Ok(())
    }

    fn reset_to_default(&self) -> StorageResult<AppConfigRecord> {
        let mut guard = self.config.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        let default_config = AppConfigRecord::default();
        *guard = default_config.clone();
        Ok(default_config)
    }

    fn update(&self, mutate: Box<dyn FnOnce(&mut AppConfigRecord) + Send>) -> StorageResult<AppConfigRecord> {
        let mut guard = self.config.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        mutate(&mut *guard);
        Ok(guard.clone())
    }
}

/// 聚合内存存储实现 (MockStorage)
#[derive(Debug, Clone)]
pub struct MockStorage {
    hosts_repo: MockHostRepository,
    groups_repo: MockGroupRepository,
    history_repo: MockHistoryRepository,
    credentials_repo: MockCredentialRepository,
    snippets_repo: MockSnippetRepository,
    tunnels_repo: MockTunnelRepository,
    config_repo: MockConfigRepository,
}

impl Default for MockStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl MockStorage {
    /// 创建空的 MockStorage
    pub fn new() -> Self {
        let hosts_repo = MockHostRepository::new();
        let credentials_repo = MockCredentialRepository::with_hosts(hosts_repo.hosts_raw());
        Self {
            hosts_repo,
            groups_repo: MockGroupRepository::new(),
            history_repo: MockHistoryRepository::new(),
            credentials_repo,
            snippets_repo: MockSnippetRepository::new(),
            tunnels_repo: MockTunnelRepository::new(),
            config_repo: MockConfigRepository::new(),
        }
    }

    /// 创建内置完整集群与丰富主机演示资产的 MockStorage (开箱即用种子引擎)
    pub fn new_seeded() -> Self {
        let groups = vec![
            GroupRecord {
                id: "grp-prod".to_string(),
                name: "生产集群 (Production)".to_string(),
                parent_id: None,
                level: 0,
                is_expanded: true,
                sort_order: 0,
            },
            GroupRecord {
                id: "grp-k8s".to_string(),
                name: "Kubernetes 集群".to_string(),
                parent_id: Some("grp-prod".to_string()),
                level: 1,
                is_expanded: true,
                sort_order: 1,
            },
            GroupRecord {
                id: "grp-db".to_string(),
                name: "核心数据库集群".to_string(),
                parent_id: Some("grp-prod".to_string()),
                level: 1,
                is_expanded: true,
                sort_order: 2,
            },
            GroupRecord {
                id: "grp-edge".to_string(),
                name: "边缘网关与缓存".to_string(),
                parent_id: None,
                level: 0,
                is_expanded: true,
                sort_order: 3,
            },
            GroupRecord {
                id: "grp-ai".to_string(),
                name: "AI 算力集群 (GPU)".to_string(),
                parent_id: None,
                level: 0,
                is_expanded: true,
                sort_order: 4,
            },
            GroupRecord {
                id: "grp-dr".to_string(),
                name: "容灾与测试环境".to_string(),
                parent_id: None,
                level: 0,
                is_expanded: false,
                sort_order: 5,
            },
        ];

        let hosts = vec![
            HostRecord {
                id: "1".to_string(),
                name: "prod-server-01".to_string(),
                address: "192.168.1.100".to_string(),
                port: 22,
                parent_group_id: Some("grp-prod".to_string()),
                credential_id: Some("cred-prod-ed25519".to_string()),
                status: HostStatus::Online,
                ping_ms: 21,
                sort_order: 0,
                notes: "主生产业务服务".to_string(),
            },
            HostRecord {
                id: "2".to_string(),
                name: "k8s-control-plane".to_string(),
                address: "10.0.0.1".to_string(),
                port: 6443,
                parent_group_id: Some("grp-k8s".to_string()),
                credential_id: Some("cred-prod-ed25519".to_string()),
                status: HostStatus::Warning,
                ping_ms: 68,
                sort_order: 1,
                notes: "K8s 主控节点".to_string(),
            },
            HostRecord {
                id: "host-k8s-w1".to_string(),
                name: "k8s-worker-node-01".to_string(),
                address: "10.0.0.11".to_string(),
                port: 22,
                parent_group_id: Some("grp-k8s".to_string()),
                credential_id: Some("cred-prod-ed25519".to_string()),
                status: HostStatus::Online,
                ping_ms: 24,
                sort_order: 2,
                notes: "K8s 工作节点 1".to_string(),
            },
            HostRecord {
                id: "3".to_string(),
                name: "db-cluster-primary".to_string(),
                address: "10.0.1.50".to_string(),
                port: 5432,
                parent_group_id: Some("grp-db".to_string()),
                credential_id: Some("cred-bastion-pwd".to_string()),
                status: HostStatus::Online,
                ping_ms: 18,
                sort_order: 3,
                notes: "PostgreSQL 主库".to_string(),
            },
            HostRecord {
                id: "host-db-s1".to_string(),
                name: "db-cluster-standby".to_string(),
                address: "10.0.1.51".to_string(),
                port: 5432,
                parent_group_id: Some("grp-db".to_string()),
                credential_id: Some("cred-bastion-pwd".to_string()),
                status: HostStatus::Online,
                ping_ms: 20,
                sort_order: 4,
                notes: "PostgreSQL 从库".to_string(),
            },
            HostRecord {
                id: "4".to_string(),
                name: "redis-cache-shard-0".to_string(),
                address: "10.0.2.10".to_string(),
                port: 6379,
                parent_group_id: Some("grp-edge".to_string()),
                credential_id: Some("cred-1pwd-agent".to_string()),
                status: HostStatus::Online,
                ping_ms: 12,
                sort_order: 5,
                notes: "Redis 缓存分片".to_string(),
            },
            HostRecord {
                id: "5".to_string(),
                name: "auth-gateway-edge".to_string(),
                address: "47.98.12.33".to_string(),
                port: 443,
                parent_group_id: Some("grp-edge".to_string()),
                credential_id: Some("cred-1pwd-agent".to_string()),
                status: HostStatus::Online,
                ping_ms: 35,
                sort_order: 6,
                notes: "边缘认证网关".to_string(),
            },
            HostRecord {
                id: "6".to_string(),
                name: "ai-inference-gpu".to_string(),
                address: "10.0.8.200".to_string(),
                port: 22,
                parent_group_id: Some("grp-ai".to_string()),
                credential_id: Some("cred-dev-rsa".to_string()),
                status: HostStatus::Online,
                ping_ms: 14,
                sort_order: 7,
                notes: "NVIDIA H100 推理卡".to_string(),
            },
            HostRecord {
                id: "7".to_string(),
                name: "backup-node-dr".to_string(),
                address: "192.168.100.250".to_string(),
                port: 22,
                parent_group_id: Some("grp-dr".to_string()),
                credential_id: Some("cred-openssh-agent".to_string()),
                status: HostStatus::Offline,
                ping_ms: 0,
                sort_order: 8,
                notes: "冷备容灾节点".to_string(),
            },
            HostRecord {
                id: "host-staging".to_string(),
                name: "staging-api-test".to_string(),
                address: "10.0.12.88".to_string(),
                port: 22,
                parent_group_id: Some("grp-dr".to_string()),
                credential_id: Some("cred-bitwarden-agent".to_string()),
                status: HostStatus::Offline,
                ping_ms: 0,
                sort_order: 9,
                notes: "预发接口测试机".to_string(),
            },
        ];

        // 预设开箱即用的真实历史会话种子
        let now_sec = 1725019200u64; // 基准时间戳
        let history = vec![
            HistoryRecord {
                id: "hist-seed-1".to_string(),
                host_id: Some("1".to_string()),
                title: "prod-server-01".to_string(),
                address: "192.168.1.100:22".to_string(),
                port: 22,
                username: "root".to_string(),
                session_type: "ssh".to_string(),
                connected_at: now_sec - 600, // 10分钟前
                disconnected_at: None,
                duration_secs: 600,
                exit_status: "active".to_string(),
                error_msg: None,
                is_pinned: true,
                connect_count: 18,
                has_snapshot: true,
                snapshot_lines: 16,
            },
            HistoryRecord {
                id: "hist-seed-7".to_string(),
                host_id: Some("7".to_string()),
                title: "ci-runner-master".to_string(),
                address: "10.0.3.10:22".to_string(),
                port: 22,
                username: "gitlab-runner".to_string(),
                session_type: "ssh".to_string(),
                connected_at: now_sec - 1800, // 30分钟前
                disconnected_at: Some(now_sec - 1350),
                duration_secs: 450,
                exit_status: "success".to_string(),
                error_msg: None,
                is_pinned: true,
                connect_count: 24,
                has_snapshot: true,
                snapshot_lines: 10,
            },
            HistoryRecord {
                id: "hist-seed-2".to_string(),
                host_id: Some("5".to_string()),
                title: "auth-gateway-edge".to_string(),
                address: "47.98.12.33:443".to_string(),
                port: 443,
                username: "admin".to_string(),
                session_type: "ssh".to_string(),
                connected_at: now_sec - 3600, // 1小时前
                disconnected_at: Some(now_sec - 2700),
                duration_secs: 900,
                exit_status: "success".to_string(),
                error_msg: None,
                is_pinned: false,
                connect_count: 5,
                has_snapshot: true,
                snapshot_lines: 12,
            },
            HistoryRecord {
                id: "hist-seed-3".to_string(),
                host_id: Some("3".to_string()),
                title: "db-cluster-primary".to_string(),
                address: "10.0.1.50:5432".to_string(),
                port: 5432,
                username: "postgres".to_string(),
                session_type: "ssh".to_string(),
                connected_at: now_sec - 7200, // 2小时前
                disconnected_at: Some(now_sec - 3600),
                duration_secs: 3600,
                exit_status: "success".to_string(),
                error_msg: None,
                is_pinned: false,
                connect_count: 12,
                has_snapshot: true,
                snapshot_lines: 15,
            },
            HistoryRecord {
                id: "hist-seed-4".to_string(),
                host_id: Some("2".to_string()),
                title: "k8s-control-plane".to_string(),
                address: "10.0.0.1:6443".to_string(),
                port: 6443,
                username: "admin".to_string(),
                session_type: "ssh".to_string(),
                connected_at: now_sec - 86400, // 昨天
                disconnected_at: Some(now_sec - 85200),
                duration_secs: 1200,
                exit_status: "success".to_string(),
                error_msg: None,
                is_pinned: false,
                connect_count: 7,
                has_snapshot: true,
                snapshot_lines: 11,
            },
            HistoryRecord {
                id: "hist-seed-5".to_string(),
                host_id: Some("4".to_string()),
                title: "redis-cache-shard-0".to_string(),
                address: "10.0.2.10:6379".to_string(),
                port: 6379,
                username: "dev".to_string(),
                session_type: "ssh".to_string(),
                connected_at: now_sec - 90000, // 昨天
                disconnected_at: Some(now_sec - 89998),
                duration_secs: 2,
                exit_status: "timeout".to_string(),
                error_msg: Some("连接超时: 目标主机网络无响应 (ETIMEDOUT)".to_string()),
                is_pinned: false,
                connect_count: 2,
                has_snapshot: false,
                snapshot_lines: 0,
            },
            HistoryRecord {
                id: "hist-seed-8".to_string(),
                host_id: None,
                title: "bastion-jump-server".to_string(),
                address: "114.55.88.99:2222".to_string(),
                port: 2222,
                username: "ops".to_string(),
                session_type: "ssh".to_string(),
                connected_at: now_sec - 100000, // 昨天
                disconnected_at: Some(now_sec - 94600),
                duration_secs: 5400,
                exit_status: "success".to_string(),
                error_msg: None,
                is_pinned: false,
                connect_count: 3,
                has_snapshot: false,
                snapshot_lines: 0,
            },
            HistoryRecord {
                id: "hist-seed-6".to_string(),
                host_id: Some("6".to_string()),
                title: "ai-inference-gpu".to_string(),
                address: "10.0.8.200:22".to_string(),
                port: 22,
                username: "cuda".to_string(),
                session_type: "ssh".to_string(),
                connected_at: now_sec - 259200, // 3天前
                disconnected_at: Some(now_sec - 252000),
                duration_secs: 7200,
                exit_status: "success".to_string(),
                error_msg: None,
                is_pinned: false,
                connect_count: 9,
                has_snapshot: true,
                snapshot_lines: 14,
            },
            HistoryRecord {
                id: "hist-seed-9".to_string(),
                host_id: None,
                title: "dev-sandbox-container".to_string(),
                address: "192.168.10.5:22".to_string(),
                port: 22,
                username: "developer".to_string(),
                session_type: "ssh".to_string(),
                connected_at: now_sec - 345600, // 4天前
                disconnected_at: Some(now_sec - 345300),
                duration_secs: 300,
                exit_status: "auth_failed".to_string(),
                error_msg: Some("SSH 密钥认证被拒绝 (Permission denied - publickey)".to_string()),
                is_pinned: false,
                connect_count: 1,
                has_snapshot: false,
                snapshot_lines: 0,
            },
            HistoryRecord {
                id: "hist-seed-10".to_string(),
                host_id: None,
                title: "backup-nas-storage".to_string(),
                address: "192.168.1.250:22".to_string(),
                port: 22,
                username: "backup".to_string(),
                session_type: "ssh".to_string(),
                connected_at: now_sec - 432000, // 5天前
                disconnected_at: Some(now_sec - 417600),
                duration_secs: 14400,
                exit_status: "success".to_string(),
                error_msg: None,
                is_pinned: false,
                connect_count: 4,
                has_snapshot: true,
                snapshot_lines: 9,
            },
        ];

        // 真实且具代表性的终端屏幕快照文本
        let mut snapshots = std::collections::HashMap::new();
        snapshots.insert(
            "hist-seed-1".to_string(),
            r#"Linux prod-server-01 5.15.0-1031-aws #35-Ubuntu SMP Fri Jan 24 16:30:11 UTC 2026 x86_64
Welcome to Ubuntu 22.04.4 LTS (GNU/Linux 5.15.0-1031-aws x86_64)

 * Documentation:  https://help.ubuntu.com
 * Management:     https://landscape.canonical.com
 * Support:        https://ubuntu.com/pro

root@prod-server-01:~# uptime
 20:30:15 up 42 days,  3:14,  1 user,  load average: 0.24, 0.18, 0.12
root@prod-server-01:~# systemctl status nginx
● nginx.service - A high performance web server and a reverse proxy server
     Loaded: loaded (/lib/systemd/system/nginx.service; enabled; vendor preset: enabled)
     Active: active (running) since Wed 2026-08-19 10:12:04 CST; 11 days ago
   Main PID: 14829 (nginx)
      Tasks: 9 (limit: 38241)
     Memory: 48.2M
        CPU: 1min 24.120s
root@prod-server-01:~# tail -n 3 /var/log/nginx/access.log
192.168.1.15 - - [30/Aug/2026:20:29:45 +0800] "GET /api/v1/health HTTP/1.1" 200 45 "-" "curl/7.81.0"
192.168.1.18 - - [30/Aug/2026:20:29:48 +0800] "POST /api/v1/metrics HTTP/1.1" 200 128 "-" "Prometheus/2.45.0"
192.168.1.20 - - [30/Aug/2026:20:29:50 +0800] "GET /api/v1/nodes HTTP/1.1" 200 1024 "-" "smalux-agent/0.1.0""#.to_string(),
        );

        snapshots.insert(
            "hist-seed-7".to_string(),
            r#"gitlab-runner@ci-runner-master:~$ docker info | grep "Server Version"
 Server Version: 26.1.3
gitlab-runner@ci-runner-master:~$ gitlab-runner status
Runtime platform                                    arch=amd64 os=linux pid=1892 revision=08101416 version=16.11.0
gitlab-runner: Service is running!
gitlab-runner@ci-runner-master:~$ gitlab-runner run-single --builds-dir /tmp/builds
Checking for jobs... nothing
gitlab-runner@ci-runner-master:~$ exit
logout"#.to_string(),
        );

        snapshots.insert(
            "hist-seed-2".to_string(),
            r#"admin@auth-gateway-edge:~$ sudo iptables -L -n -v --line-numbers
Chain INPUT (policy ACCEPT 1420K packets, 189M bytes)
num   pkts bytes target     prot opt in     out     source               destination         
1     982K  132M ACCEPT     tcp  --  eth0   *       0.0.0.0/0            0.0.0.0/0            tcp dpt:443
2     124K   18M ACCEPT     tcp  --  eth0   *       0.0.0.0/0            0.0.0.0/0            tcp dpt:80
admin@auth-gateway-edge:~$ curl -I http://127.0.0.1:8080/health
HTTP/1.1 200 OK
Date: Sun, 30 Aug 2026 19:40:02 GMT
Content-Type: application/json
Content-Length: 18
Server: smalux-gateway/2.4.0

{"status":"healthy"}
admin@auth-gateway-edge:~$ exit
logout"#.to_string(),
        );

        snapshots.insert(
            "hist-seed-3".to_string(),
            r#"postgres@db-cluster-primary:~$ psql -U postgres -d smalux_db
psql (16.2 (Debian 16.2-1.pgdg120+1))
Type "help" for help.

smalux_db=# SELECT datname, numbackends, xact_commit, xact_rollback FROM pg_stat_database WHERE datname='smalux_db';
  datname  | numbackends | xact_commit | xact_rollback 
-----------+-------------+-------------+---------------
 smalux_db |          16 |     4892182 |           142
(1 row)

smalux_db=# SELECT count(*) FROM hosts_inventory;
 count 
-------
   128
(1 row)

smalux_db=# \q
postgres@db-cluster-primary:~$ exit
logout"#.to_string(),
        );

        snapshots.insert(
            "hist-seed-4".to_string(),
            r#"admin@k8s-control-plane:~$ kubectl get nodes -o wide
NAME           STATUS   ROLES           AGE   VERSION   INTERNAL-IP   OS-IMAGE             KERNEL-VERSION
k8s-master-1   Ready    control-plane   85d   v1.29.2   10.0.0.1      Ubuntu 22.04.3 LTS   5.15.0-94-generic
k8s-worker-1   Ready    <none>          85d   v1.29.2   10.0.0.11     Ubuntu 22.04.3 LTS   5.15.0-94-generic
k8s-worker-2   Ready    <none>          85d   v1.29.2   10.0.0.12     Ubuntu 22.04.3 LTS   5.15.0-94-generic
admin@k8s-control-plane:~$ kubectl get pods -n kube-system
NAME                                       READY   STATUS    RESTARTS   AGE
coredns-76f75df574-8k9pl                   1/1     Running   0          85d
etcd-k8s-master-1                          1/1     Running   0          85d
kube-apiserver-k8s-master-1                1/1     Running   0          85d
kube-controller-manager-k8s-master-1       1/1     Running   0          85d
admin@k8s-control-plane:~$ exit
logout"#.to_string(),
        );

        snapshots.insert(
            "hist-seed-6".to_string(),
            r#"cuda@ai-inference-gpu:~$ nvidia-smi
Sun Aug 30 18:22:10 2026       
+-----------------------------------------------------------------------------------------+
| NVIDIA-SMI 550.54.14              Driver Version: 550.54.14      CUDA Version: 12.4     |
|-----------------------------------------+------------------------+----------------------+
| GPU  Name                 Persistence-M | Bus-Id          Disp.A | Volatile Uncorr. ECC |
| Fan  Temp   Perf          Pwr:Usage/Cap |           Memory-Usage | GPU-Util  Compute M. |
|=========================================+========================+======================|
|   0  NVIDIA H100 80GB HBM3          On  | 00000000:06:00.0   Off |                    0 |
| N/A   42C    P0             112W / 700W |  42100MiB / 81559MiB |    68%      Default |
+-----------------------------------------+------------------------+----------------------+
cuda@ai-inference-gpu:~$ docker ps --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
NAMES           STATUS         PORTS
vllm-qwen-72b   Up 2 days      0.0.0.0:8000->8000/tcp
cuda@ai-inference-gpu:~$ exit
logout"#.to_string(),
        );

        snapshots.insert(
            "hist-seed-10".to_string(),
            r#"backup@backup-nas-storage:~$ df -h /mnt/data
Filesystem      Size  Used Avail Use% Mounted on
/dev/md0         18T  9.4T  7.8T  55% /mnt/data
backup@backup-nas-storage:~$ zpool status -x
all pools are healthy
backup@backup-nas-storage:~$ rsync --version | head -n 1
rsync  version 3.2.7  protocol version 31
backup@backup-nas-storage:~$ exit
logout"#.to_string(),
        );

        let credentials = vec![
            CredentialRecord {
                id: "cred-prod-ed25519".to_string(),
                name: "生产集群 Ed25519 密钥".to_string(),
                cred_type: CredentialType::Key,
                algorithm: "Ed25519".to_string(),
                username: Some("root".to_string()),
                secret_data: "-----BEGIN OPENSSH PRIVATE KEY-----\nb3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\nQyNTUxOQAAACDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6SQAAAJCR2Y69kdmO\nvQAAAAtzc2gtZWQyNTUxOQAAACDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6\nSQAAAEA6WjG4m2JpL5kZ8yQ3uP9tL3wR2bN6pG8oP4qM7lX2nDH8g20vX7K9p1BfN2wP\n4lXqZbM4xGgA9QJ6tL7r1n6SQAAAA1zbWFsdXgtc3NoLWtleQECAwQ=\n-----END OPENSSH PRIVATE KEY-----".to_string(),
                passphrase: Some("••••••••".to_string()),
                public_key: Some("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMfyDbS9fsr2nUF83bA/iVeplszjEaAD1Anq0vuvWfpJ root@smalux-k8s-prod".to_string()),
                fingerprint: Some("SHA256:k9x8Ym+3pLq1G7vX2nR8uM4aP9tL3wQ2bN6pG8oP4qM".to_string()),
                bound_host_count: 5,
                created_at: "2026-08-15 10:20:00".to_string(),
                updated_at: "2026-09-01 09:15:00".to_string(),
                notes: "Kubernetes 核心控制面与网关认证主密钥".to_string(),
            },
            CredentialRecord {
                id: "cred-bastion-pwd".to_string(),
                name: "堡垒跳板机 Root 管理密码".to_string(),
                cred_type: CredentialType::Password,
                algorithm: "Password".to_string(),
                username: Some("root".to_string()),
                secret_data: "SmaluxSecure#2026!P@ss".to_string(),
                passphrase: None,
                public_key: None,
                fingerprint: None,
                bound_host_count: 2,
                created_at: "2026-08-18 14:30:00".to_string(),
                updated_at: "2026-08-30 18:00:00".to_string(),
                notes: "边缘网关与跳板机应急控制台特权密码".to_string(),
            },
            CredentialRecord {
                id: "cred-1pwd-agent".to_string(),
                name: "1Password SSH Agent".to_string(),
                cred_type: CredentialType::Agent,
                algorithm: "1Password".to_string(),
                username: Some("developer".to_string()),
                secret_data: r"\\.\pipe\1password-ssh-agent".to_string(),
                passphrase: None,
                public_key: Some("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIPq8Xm7kL9vN2wA4bF6jZ8qM3uP9tL3wR2bN6pG8oP4q 1password-agent".to_string()),
                fingerprint: Some("SHA256:1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d".to_string()),
                bound_host_count: 3,
                created_at: "2026-08-20 16:45:00".to_string(),
                updated_at: "2026-09-01 11:30:00".to_string(),
                notes: "硬件安全保管箱，受 Windows Hello 生物识别保护".to_string(),
            },
            CredentialRecord {
                id: "cred-dev-rsa".to_string(),
                name: "CI/CD 流水线 RSA 密钥".to_string(),
                cred_type: CredentialType::Key,
                algorithm: "RSA-4096".to_string(),
                username: Some("gitlab-runner".to_string()),
                secret_data: "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEA0k6K9X7L9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6SQ2Y69kd\nvQMwAAAAtzc2gtcnNhAAAAAwEAAQAAAgEAv7b4a2p8zXqN3vP9xK2m4rL9nO1pQ8tL\n3wR2bN6pG8oP4qM7lX2nDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6SQ\nA6WjG4m2JpL5kZ8yQ3uP9tL3wR2bN6pG8oP4qM7lX2nDH8g20vX7K9p1BfN2wP4l\nXqZbM4xGgA9QJ6tL7r1n6SQAAAA1zbWFsdXgtc3NoLWtleQECAwQFAgcICQoLDA0O\nDxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4vMDEyMzQ1Njc4OTo7PD0+P0\nBBQkNERUZHSElKS0xNTk9QUVJTVFVWV1hZWltcXV5fYGFiY2RlZmdoaWprbG1ub3Bx\ncnN0dXZ3eHl6e3x9fn+AgYKDhIWGh4iJiouMjY6PkJGSk5SVlpeYmZqbnJ2en6Ch\noqOkpaanqKmqq6ytrq+wsbKztLW2t7i5uru8vb6/wMHCw8TFxsfIycrLzM3Oz9DR\n0tPU1dbX2Nna29zd3t/g4eLj5OXm5+jp6uvs7e7v8PHy8/T19vf4+fr7/P3+/wID\n-----END RSA PRIVATE KEY-----".to_string(),
                passphrase: None,
                public_key: Some("ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAACAQDv7b4a2p8zXqN3vP9xK2m4rL9nO1pQ8tL3wR2bN6pG8oP4qM7lX2nDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6SQA6WjG4m2JpL5kZ8yQ3uP9tL3wR2bN6pG8oP4qM7lX2nDH8g20vX7K9p1BfN2wP4lXqZbM4xGgA9QJ6tL7r1n6SQAAAA1zbWFsdXgtc3NoLWtleQECAwQ gitlab@runner".to_string()),
                fingerprint: Some("SHA256:9c8b7a6f5e4d3c2b1a0f9e8d7c6b5a4f".to_string()),
                bound_host_count: 1,
                created_at: "2026-08-25 09:00:00".to_string(),
                updated_at: "2026-08-25 09:00:00".to_string(),
                notes: "GitLab Runner 持续部署构建机专有免密凭据".to_string(),
            },
            CredentialRecord {
                id: "cred-openssh-agent".to_string(),
                name: "Windows OpenSSH Agent".to_string(),
                cred_type: CredentialType::Agent,
                algorithm: "OpenSSH".to_string(),
                username: Some("ssh-agent".to_string()),
                secret_data: r"\\.\pipe\openssh-ssh-agent".to_string(),
                passphrase: None,
                public_key: Some("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIPq8Xm7kL9vN2wA4bF6jZ8qM3uP9tL3wR2bN6pG8oP4q openssh-agent".to_string()),
                fingerprint: Some("SHA256:8f4e2c9a1b3d5e7f0a2c4e6b8d0f1a3c".to_string()),
                bound_host_count: 4,
                created_at: "2026-08-22 11:00:00".to_string(),
                updated_at: "2026-08-22 11:00:00".to_string(),
                notes: "Windows 内置 OpenSSH Authentication Agent 命名管道".to_string(),
            },
            CredentialRecord {
                id: "cred-bitwarden-agent".to_string(),
                name: "Bitwarden SSH Agent".to_string(),
                cred_type: CredentialType::Agent,
                algorithm: "Bitwarden".to_string(),
                username: Some("vault".to_string()),
                secret_data: r"\\.\pipe\bitwarden-ssh-agent".to_string(),
                passphrase: None,
                public_key: Some("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIPq8Xm7kL9vN2wA4bF6jZ8qM3uP9tL3wR2bN6pG8oP4q bitwarden-agent".to_string()),
                fingerprint: Some("SHA256:3d5e7f9a1c2b4d6e8f0a1b3c5d7e9f1a".to_string()),
                bound_host_count: 2,
                created_at: "2026-08-28 15:20:00".to_string(),
                updated_at: "2026-08-28 15:20:00".to_string(),
                notes: "Bitwarden / Vaultwarden 桌面端安全托管 SSH Agent".to_string(),
            },
        ];

        let snippet_groups = vec![
            SnippetGroupRecord {
                id: "sgrp-docker".to_string(),
                name: "Docker 容器与微服务".to_string(),
                parent_id: None,
                level: 0,
                is_expanded: true,
                sort_order: 0,
            },
            SnippetGroupRecord {
                id: "sgrp-k8s".to_string(),
                name: "Kubernetes 集群排障".to_string(),
                parent_id: None,
                level: 0,
                is_expanded: true,
                sort_order: 1,
            },
            SnippetGroupRecord {
                id: "sgrp-ops".to_string(),
                name: "Linux 系统基础运维".to_string(),
                parent_id: None,
                level: 0,
                is_expanded: true,
                sort_order: 2,
            },
            SnippetGroupRecord {
                id: "sgrp-ops-net".to_string(),
                name: "网络与端口诊断".to_string(),
                parent_id: Some("sgrp-ops".to_string()),
                level: 1,
                is_expanded: true,
                sort_order: 0,
            },
            SnippetGroupRecord {
                id: "sgrp-db".to_string(),
                name: "数据库维护与慢查询".to_string(),
                parent_id: None,
                level: 0,
                is_expanded: true,
                sort_order: 3,
            },
        ];

        let snippets = vec![
            SnippetRecord {
                id: "snip-docker-ps".to_string(),
                parent_group_id: Some("sgrp-docker".to_string()),
                title: "Docker 容器健康列表".to_string(),
                content: "docker ps --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}\t{{.Image}}'".to_string(),
                language: "bash".to_string(),
                tags: vec!["docker".to_string(), "status".to_string()],
                auto_execute: true,
                description: "格式化列出所有正在运行的 Docker 容器及其映射端口".to_string(),
                is_favorite: true,
                sort_order: 0,
                updated_at: "2026-09-01 10:00:00".to_string(),
            },
            SnippetRecord {
                id: "snip-docker-log".to_string(),
                parent_group_id: Some("sgrp-docker".to_string()),
                title: "Docker 实时日志追踪".to_string(),
                content: "docker logs -f --tail={{lines:100}} {{container_name}}".to_string(),
                language: "bash".to_string(),
                tags: vec!["docker".to_string(), "logs".to_string()],
                auto_execute: true,
                description: "动态参数化追踪指定容器尾部日志流".to_string(),
                is_favorite: true,
                sort_order: 1,
                updated_at: "2026-09-01 10:15:00".to_string(),
            },
            SnippetRecord {
                id: "snip-docker-prune".to_string(),
                parent_group_id: Some("sgrp-docker".to_string()),
                title: "清理悬空镜像与未用卷".to_string(),
                content: "docker system prune -f --volumes".to_string(),
                language: "bash".to_string(),
                tags: vec!["docker".to_string(), "cleanup".to_string()],
                auto_execute: false,
                description: "安全释放 Docker 废弃卷与虚悬镜像存储空间 (仅粘贴需手动确认)".to_string(),
                is_favorite: false,
                sort_order: 2,
                updated_at: "2026-08-30 16:00:00".to_string(),
            },
            SnippetRecord {
                id: "snip-k8s-abnormal".to_string(),
                parent_group_id: Some("sgrp-k8s".to_string()),
                title: "K8s 全命名空间异常 Pod 排查".to_string(),
                content: "kubectl get pods -A --field-selector=status.phase!=Running,status.phase!=Succeeded".to_string(),
                language: "bash".to_string(),
                tags: vec!["k8s".to_string(), "pod".to_string(), "troubleshoot".to_string()],
                auto_execute: true,
                description: "快速筛出集群中 CrashLoopBackOff 或 Pending 状态的故障 Pod".to_string(),
                is_favorite: true,
                sort_order: 0,
                updated_at: "2026-08-28 14:00:00".to_string(),
            },
            SnippetRecord {
                id: "snip-k8s-restart".to_string(),
                parent_group_id: Some("sgrp-k8s".to_string()),
                title: "K8s 滚动重启 Deployment".to_string(),
                content: "kubectl rollout restart deployment/{{deployment_name}} -n {{namespace:default}}".to_string(),
                language: "bash".to_string(),
                tags: vec!["k8s".to_string(), "restart".to_string()],
                auto_execute: true,
                description: "无损滚动重启指定的 Kubernetes 工作负载".to_string(),
                is_favorite: false,
                sort_order: 1,
                updated_at: "2026-08-29 11:30:00".to_string(),
            },
            SnippetRecord {
                id: "snip-sys-load".to_string(),
                parent_group_id: Some("sgrp-ops".to_string()),
                title: "Linux CPU 与内存负载 Top 20".to_string(),
                content: "top -b -n 1 | head -n 20".to_string(),
                language: "bash".to_string(),
                tags: vec!["linux".to_string(), "performance".to_string()],
                auto_execute: true,
                description: "单次截取系统负载、任务队列与前 20 活跃进程".to_string(),
                is_favorite: false,
                sort_order: 0,
                updated_at: "2026-08-25 09:00:00".to_string(),
            },
            SnippetRecord {
                id: "snip-disk-large".to_string(),
                parent_group_id: Some("sgrp-ops".to_string()),
                title: "磁盘大文件扫描 (>500MB)".to_string(),
                content: "find {{scan_path:/var/log}} -type f -size +{{size_mb:500}}M -exec ls -lh {} + 2>/dev/null | awk '{print $9 \": \" $5}'".to_string(),
                language: "bash".to_string(),
                tags: vec!["disk".to_string(), "find".to_string()],
                auto_execute: true,
                description: "排查磁盘爆满根因，快速定位指定目录下超过阈值的大文件".to_string(),
                is_favorite: true,
                sort_order: 1,
                updated_at: "2026-08-26 15:40:00".to_string(),
            },
            SnippetRecord {
                id: "snip-net-port".to_string(),
                parent_group_id: Some("sgrp-ops-net".to_string()),
                title: "查询指定端口占用与监听进程".to_string(),
                content: "lsof -i :{{port:8080}} || netstat -tulnp | grep :{{port:8080}}".to_string(),
                language: "bash".to_string(),
                tags: vec!["network".to_string(), "port".to_string()],
                auto_execute: true,
                description: "检测本地端口监听状态与绑定 PID 进程名".to_string(),
                is_favorite: true,
                sort_order: 0,
                updated_at: "2026-08-27 18:00:00".to_string(),
            },
            SnippetRecord {
                id: "snip-db-mysql-process".to_string(),
                parent_group_id: Some("sgrp-db".to_string()),
                title: "MySQL 活跃长事务与锁排查".to_string(),
                content: "mysql -u {{user:root}} -p{{password}} -h {{host:127.0.0.1}} -e 'SHOW FULL PROCESSLIST;'".to_string(),
                language: "sql".to_string(),
                tags: vec!["mysql".to_string(), "database".to_string()],
                auto_execute: false,
                description: "查看 MySQL 当前执行超过阈值的慢查询与卡顿连接".to_string(),
                is_favorite: false,
                sort_order: 0,
                updated_at: "2026-08-28 20:00:00".to_string(),
            },
            SnippetRecord {
                id: "snip-db-redis-info".to_string(),
                parent_group_id: Some("sgrp-db".to_string()),
                title: "Redis 内存占用分析".to_string(),
                content: "redis-cli -h {{host:127.0.0.1}} -p {{port:6379}} info memory | grep -E 'used_memory_human|used_memory_peak_human|mem_fragmentation_ratio'".to_string(),
                language: "bash".to_string(),
                tags: vec!["redis".to_string(), "memory".to_string()],
                auto_execute: true,
                description: "获取 Redis 当前内存峰值与碎片率指标".to_string(),
                is_favorite: true,
                sort_order: 1,
                updated_at: "2026-08-30 09:20:00".to_string(),
            },
        ];

        let tunnels = vec![
            TunnelRecord {
                id: "tun-mysql-prod".to_string(),
                name: "生产 MySQL 数据库直连".to_string(),
                tunnel_type: TunnelType::Local,
                ssh_host_id: Some("host-prod-01".to_string()),
                ssh_host_name: "prod-server-01".to_string(),
                local_bind: "127.0.0.1".to_string(),
                local_port: 3306,
                remote_host: "10.0.0.8".to_string(),
                remote_port: 3306,
                jump_chain: Vec::new(),
                is_running: true,
                auto_start: true,
                auto_reconnect: true,
                remote_dns: false,
                compression: true,
                active_connections: 3,
                total_bytes_in: 25 * 1024 * 1024 + 400 * 1024,
                total_bytes_out: 4 * 1024 * 1024 + 128 * 1024,
                proxy_proto: String::new(),
                proxy_username: String::new(),
                proxy_password: String::new(),
                notes: "将远程隔离区生产主数据库映射至本地 3306 端口，供本地 Navicat 安全连接".to_string(),
                updated_at: "2026-09-01 10:20:00".to_string(),
            },
            TunnelRecord {
                id: "tun-redis-k8s".to_string(),
                name: "K8s Redis 哨兵集群映射".to_string(),
                tunnel_type: TunnelType::Local,
                ssh_host_id: Some("host-k8s-m1".to_string()),
                ssh_host_name: "k8s-master-01".to_string(),
                local_bind: "127.0.0.1".to_string(),
                local_port: 6379,
                remote_host: "10.244.2.15".to_string(),
                remote_port: 6379,
                jump_chain: Vec::new(),
                is_running: true,
                auto_start: true,
                auto_reconnect: true,
                remote_dns: false,
                compression: false,
                active_connections: 1,
                total_bytes_in: 8 * 1024 * 1024,
                total_bytes_out: 1024 * 1024,
                proxy_proto: String::new(),
                proxy_username: String::new(),
                proxy_password: String::new(),
                notes: "用于本地查看与排查集群缓存热点 Key 与集群状态".to_string(),
                updated_at: "2026-08-31 16:45:00".to_string(),
            },
            TunnelRecord {
                id: "tun-webhook-dev".to_string(),
                name: "公网穿透本地 Webhook 调试".to_string(),
                tunnel_type: TunnelType::Remote,
                ssh_host_id: Some("host-gateway".to_string()),
                ssh_host_name: "gateway-bastion".to_string(),
                local_bind: "127.0.0.1".to_string(),
                local_port: 3000,
                remote_host: "0.0.0.0".to_string(),
                remote_port: 8080,
                jump_chain: Vec::new(),
                is_running: false,
                auto_start: false,
                auto_reconnect: true,
                remote_dns: false,
                compression: true,
                active_connections: 0,
                total_bytes_in: 512 * 1024,
                total_bytes_out: 1024 * 1024 * 2,
                proxy_proto: String::new(),
                proxy_username: String::new(),
                proxy_password: String::new(),
                notes: "将公网网关 8080 端口打回本地 Node/Rust 开发服务 3000 端口，接收第三方回调".to_string(),
                updated_at: "2026-08-30 18:00:00".to_string(),
            },
            TunnelRecord {
                id: "tun-socks5-corp".to_string(),
                name: "生产全网段 SOCKS5 代理".to_string(),
                tunnel_type: TunnelType::Dynamic,
                ssh_host_id: Some("host-prod-01".to_string()),
                ssh_host_name: "prod-server-01".to_string(),
                local_bind: "127.0.0.1".to_string(),
                local_port: 1080,
                remote_host: "ANY".to_string(),
                remote_port: 0,
                jump_chain: Vec::new(),
                is_running: true,
                auto_start: false,
                auto_reconnect: true,
                remote_dns: true,
                compression: true,
                active_connections: 8,
                total_bytes_in: 1024 * 1024 * 128,
                total_bytes_out: 1024 * 1024 * 18,
                proxy_proto: String::new(),
                proxy_username: String::new(),
                proxy_password: String::new(),
                notes: "本地透明 SOCKS5 代理网关，支持浏览器或终端走内网网段统一访问".to_string(),
                updated_at: "2026-09-02 08:30:00".to_string(),
            },
            TunnelRecord {
                id: "tun-bastion-chain".to_string(),
                name: "内网第二跳核心堡垒机".to_string(),
                tunnel_type: TunnelType::JumpHost,
                ssh_host_id: Some("host-gateway".to_string()),
                ssh_host_name: "gateway-bastion".to_string(),
                local_bind: "".to_string(),
                local_port: 0,
                remote_host: "10.100.0.1".to_string(),
                remote_port: 22,
                jump_chain: vec![
                    crate::domain::tunnel::JumpHopRecord {
                        host_id: "host-gateway".to_string(),
                        host_name: "gateway-bastion".to_string(),
                        host_address: "10.10.1.1".to_string(),
                        host_port: 22,
                        enabled: true,
                    },
                    crate::domain::tunnel::JumpHopRecord {
                        host_id: "host-prod-01".to_string(),
                        host_name: "prod-server-01".to_string(),
                        host_address: "10.100.0.1".to_string(),
                        host_port: 22,
                        enabled: true,
                    },
                ],
                is_running: true,
                auto_start: true,
                auto_reconnect: true,
                remote_dns: false,
                compression: false,
                active_connections: 2,
                total_bytes_in: 1024 * 1024 * 5,
                total_bytes_out: 1024 * 1024 * 3,
                proxy_proto: String::new(),
                proxy_username: String::new(),
                proxy_password: String::new(),
                notes: "级联跳板机 (ProxyJump)，作为访问深度内网物理隔离集群的中转桥梁".to_string(),
                updated_at: "2026-08-29 14:00:00".to_string(),
            },
            TunnelRecord {
                id: "tun-proxy-corp".to_string(),
                name: "公司统一出网 HTTP/SOCKS 代理".to_string(),
                tunnel_type: TunnelType::ProxyServer,
                ssh_host_id: None,
                ssh_host_name: "proxy.internal.corp".to_string(),
                local_bind: "127.0.0.1".to_string(),
                local_port: 7890,
                remote_host: "proxy.corp.net".to_string(),
                remote_port: 7890,
                jump_chain: Vec::new(),
                is_running: true,
                auto_start: false,
                auto_reconnect: true,
                remote_dns: true,
                compression: false,
                active_connections: 5,
                total_bytes_in: 1024 * 1024 * 45,
                total_bytes_out: 1024 * 1024 * 12,
                proxy_proto: "SOCKS5".to_string(),
                proxy_username: "corp_user".to_string(),
                proxy_password: "corp_password".to_string(),
                notes: "出网专用代理服务，支持 HTTP 与 SOCKS5 双协议转发".to_string(),
                updated_at: "2026-09-02 09:15:00".to_string(),
            },
        ];

        tracing::info!(
            target: "smagical_core::storage",
            "初始化 MockStorage 预设种子引擎完成: 已注入 {} 个层级分组, {} 台主机资产, {} 条历史会话记录 (含 {} 份屏幕快照), {} 条凭据记录, {} 个代码片段分组, {} 个代码片段与 {} 条网络隧道/代理",
            groups.len(),
            hosts.len(),
            history.len(),
            snapshots.len(),
            credentials.len(),
            snippet_groups.len(),
            snippets.len(),
            tunnels.len()
        );

        let hosts_repo = MockHostRepository::with_hosts(hosts);
        let credentials_repo = MockCredentialRepository::with_credentials_and_hosts(credentials, hosts_repo.hosts_raw());
        Self {
            hosts_repo,
            groups_repo: MockGroupRepository::with_groups(groups),
            history_repo: MockHistoryRepository::with_history_and_snapshots(history, snapshots),
            credentials_repo,
            snippets_repo: MockSnippetRepository::with_data(snippets, snippet_groups),
            tunnels_repo: MockTunnelRepository::with_tunnels(tunnels),
            config_repo: MockConfigRepository::new(),
        }
    }
}

impl AppStorage for MockStorage {

    fn hosts(&self) -> &dyn HostRepository {
        &self.hosts_repo
    }

    fn groups(&self) -> &dyn GroupRepository {
        &self.groups_repo
    }

    fn history(&self) -> &dyn HistoryRepository {
        &self.history_repo
    }

    fn credentials(&self) -> &dyn CredentialRepository {
        &self.credentials_repo
    }

    fn snippets(&self) -> &dyn SnippetRepository {
        &self.snippets_repo
    }

    fn tunnels(&self) -> &dyn TunnelRepository {
        &self.tunnels_repo
    }

    fn config(&self) -> &dyn ConfigRepository {
        &self.config_repo
    }

    fn reload(&self) -> StorageResult<()> {
        tracing::debug!(target: "smagical_core::storage", "MockStorage 内存重新加载请求 (无操作)");
        Ok(())
    }

    fn flush(&self) -> StorageResult<()> {
        tracing::debug!(target: "smagical_core::storage", "MockStorage 内存刷盘请求 (无操作)");
        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_storage_seeded_data() {
        let storage = MockStorage::new_seeded();
        
        let groups = storage.groups().list_all().unwrap();
        assert_eq!(groups.len(), 6);
        assert_eq!(groups[0].name, "生产集群 (Production)");

        let hosts = storage.hosts().list_all().unwrap();
        assert_eq!(hosts.len(), 10);
        assert_eq!(hosts[0].name, "prod-server-01");
    }

    #[test]
    fn test_mock_storage_host_crud() {
        let storage = MockStorage::new();
        assert_eq!(storage.hosts().list_all().unwrap().len(), 0);

        let new_host = HostRecord::new("h1", "Host 1", "127.0.0.1", 22);
        storage.hosts().save(&new_host).unwrap();

        assert_eq!(storage.hosts().list_all().unwrap().len(), 1);
        let found = storage.hosts().get_by_id("h1").unwrap().unwrap();
        assert_eq!(found.name, "Host 1");

        let deleted = storage.hosts().delete("h1").unwrap();
        assert!(deleted);
        assert_eq!(storage.hosts().list_all().unwrap().len(), 0);
    }

    #[test]
    fn test_mock_storage_list_reordering() {
        let storage = MockStorage::new();
        storage.hosts().save(&HostRecord::new("1", "H1", "1.1.1.1", 22)).unwrap();
        storage.hosts().save(&HostRecord::new("2", "H2", "2.2.2.2", 22)).unwrap();
        storage.hosts().save(&HostRecord::new("3", "H3", "3.3.3.3", 22)).unwrap();

        // 调整顺序为: 3, 1, 2
        storage.hosts().update_list_order(&["3".to_string(), "1".to_string(), "2".to_string()]).unwrap();

        let hosts = storage.hosts().list_all().unwrap();
        assert_eq!(hosts[0].id, "3");
        assert_eq!(hosts[1].id, "1");
        assert_eq!(hosts[2].id, "2");
    }

    #[test]
    fn test_mock_storage_history_crud_and_pin() {
        let storage = MockStorage::new_seeded();
        let history = storage.history().list_all().unwrap();
        assert_eq!(history.len(), 10);
        // 置顶项排在最前
        assert!(history[0].is_pinned);
        assert!(history[1].is_pinned);

        // 验证种子中已包含终端屏幕快照
        let snap1 = storage.history().get_snapshot("hist-seed-1").unwrap();
        assert!(snap1.is_some());
        assert!(snap1.unwrap().contains("prod-server-01"));

        // 新建并保存一条新记录
        let mut new_hist = HistoryRecord::new_ssh(
            "test-hist-1".to_string(),
            Some("1".to_string()),
            "custom-ssh".to_string(),
            "1.2.3.4:22".to_string(),
            22,
            "root".to_string(),
            1725020000,
        );
        storage.history().save(&new_hist).unwrap();

        let list_after_save = storage.history().list_all().unwrap();
        assert_eq!(list_after_save.len(), 11);

        // 切换置顶
        let pinned = storage.history().toggle_pin("test-hist-1").unwrap();
        assert!(pinned);

        // 验证置顶后排序
        let list_after_pin = storage.history().list_all().unwrap();
        assert!(list_after_pin[0].is_pinned);

        // 标记关闭
        new_hist.mark_closed(1725020600);
        storage.history().save(&new_hist).unwrap();
        let fetched = storage.history().get_by_id("test-hist-1").unwrap().unwrap();
        assert_eq!(fetched.exit_status, "success");
        assert_eq!(fetched.duration_secs, 600);

        // 删除记录
        let deleted = storage.history().delete("test-hist-1").unwrap();
        assert!(deleted);
        assert_eq!(storage.history().list_all().unwrap().len(), 10);

        // 清空（保留置顶）
        storage.history().clear_all(true).unwrap();
        let remaining = storage.history().list_all().unwrap();
        assert_eq!(remaining.len(), 2); // 仅剩 2 个种子置顶项
        assert!(remaining.iter().all(|r| r.is_pinned));
    }


    #[test]
    fn test_mock_storage_session_snapshot() {
        let storage = MockStorage::new();
        let mut hist = HistoryRecord::new_ssh(
            "hist-snap-1".to_string(),
            None,
            "test-server".to_string(),
            "192.168.1.1:22".to_string(),
            22,
            "root".to_string(),
            1000,
        );

        // 模拟多行终端屏幕输出
        let raw_output = "line 1: welcome\nline 2: login success\nline 3: ls -la\nline 4: output 1\nline 5: exit 0";
        storage.history().save_snapshot("hist-snap-1", raw_output, 3).unwrap(); // 限制最多保留 3 行

        let snapshot = storage.history().get_snapshot("hist-snap-1").unwrap().unwrap();
        let lines: Vec<&str> = snapshot.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "line 3: ls -la");
        assert_eq!(lines[1], "line 4: output 1");
        assert_eq!(lines[2], "line 5: exit 0");

        hist.record_snapshot(3);
        storage.history().save(&hist).unwrap();

        let fetched = storage.history().get_by_id("hist-snap-1").unwrap().unwrap();
        assert!(fetched.has_snapshot);
        assert_eq!(fetched.snapshot_lines, 3);

        // 删除历史会话，快照应自动关联清除
        storage.history().delete("hist-snap-1").unwrap();
        let snap_after_del = storage.history().get_snapshot("hist-snap-1").unwrap();
        assert!(snap_after_del.is_none());
    }

    #[test]
    fn test_mock_storage_local_shell_history() {
        let storage = MockStorage::new();
        let local_hist = HistoryRecord::new_local(
            "hist-local-1".to_string(),
            Some("local-powershell".to_string()),
            "PowerShell 7".to_string(),
            "PowerShell 7 (pwsh)".to_string(),
            1725020000,
        );
        storage.history().save(&local_hist).unwrap();

        let list = storage.history().list_all().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].session_type, "local");
        assert_eq!(list[0].host_id.as_deref(), Some("local-powershell"));
        assert_eq!(list[0].address, "Local (PowerShell 7)");
    }

    #[test]
    fn test_mock_storage_credential_crud() {
        let storage = MockStorage::new_seeded();
        let list = storage.credentials().list_all().unwrap();
        assert_eq!(list.len(), 6);

        // 新增凭据
        let new_cred = CredentialRecord::new_key(
            "cred-test-key",
            "测试 Ed25519 凭据",
            "Ed25519",
            "test_private_key",
            None,
            Some("ssh-ed25519 AAAAC... test@test".to_string()),
            Some("SHA256:11223344".to_string()),
            "单元测试专供",
        );
        storage.credentials().save(&new_cred).unwrap();
        assert_eq!(storage.credentials().list_all().unwrap().len(), 7);

        // 查询单条
        let fetched = storage.credentials().get_by_id("cred-test-key").unwrap().unwrap();
        assert_eq!(fetched.name, "测试 Ed25519 凭据");
        assert_eq!(fetched.cred_type, CredentialType::Key);
        assert_eq!(fetched.fingerprint.as_deref(), Some("SHA256:11223344"));

        // 更新单条
        let mut updated = fetched;
        updated.name = "已重命名测试凭据".to_string();
        storage.credentials().save(&updated).unwrap();
        assert_eq!(storage.credentials().get_by_id("cred-test-key").unwrap().unwrap().name, "已重命名测试凭据");

        // 删除单条
        let deleted = storage.credentials().delete("cred-test-key").unwrap();
        assert!(deleted);
        assert_eq!(storage.credentials().list_all().unwrap().len(), 6);

        // 验证凭据与主机相互引用与查询
        // 1. 根据凭据查关联主机
        let prod_hosts = storage.hosts().list_by_credential("cred-prod-ed25519").unwrap();
        assert_eq!(prod_hosts.len(), 3);
        assert_eq!(prod_hosts[0].id, "1");

        // 2. 根据凭据查关联主机 ID
        let bound_ids = storage.credentials().get_bound_hosts("cred-prod-ed25519").unwrap();
        assert_eq!(bound_ids.len(), 3);
        assert!(bound_ids.contains(&"1".to_string()));
        assert!(bound_ids.contains(&"2".to_string()));
        assert!(bound_ids.contains(&"host-k8s-w1".to_string()));

        // 3. 动态引用计数验证
        let cred_prod = storage.credentials().get_by_id("cred-prod-ed25519").unwrap().unwrap();
        assert_eq!(cred_prod.bound_host_count, 3);

        // 4. 按类型分类查询
        let agent_creds = storage.credentials().list_by_type(CredentialType::Agent).unwrap();
        assert_eq!(agent_creds.len(), 3);

        // 5. 模糊搜索
        let search_results = storage.credentials().search("1Password").unwrap();
        assert_eq!(search_results.len(), 1);
        assert_eq!(search_results[0].id, "cred-1pwd-agent");
    }

    #[test]
    fn test_mock_storage_snippets_and_groups_crud() {
        let storage = MockStorage::new_seeded();

        // 1. 验证预设种子
        let groups = storage.snippets().list_groups().unwrap();
        let snippets = storage.snippets().list_all().unwrap();
        assert_eq!(groups.len(), 5);
        assert_eq!(snippets.len(), 10);

        // 2. 按分组过滤
        let docker_snippets = storage.snippets().list_by_group(Some("sgrp-docker")).unwrap();
        assert_eq!(docker_snippets.len(), 3);

        // 3. 搜索
        let search_res = storage.snippets().search("restart").unwrap();
        assert_eq!(search_res.len(), 1);
        assert_eq!(search_res[0].id, "snip-k8s-restart");

        // 4. 星标切换
        let fav_before = storage.snippets().get_by_id("snip-docker-prune").unwrap().unwrap().is_favorite;
        assert!(!fav_before);
        let fav_after = storage.snippets().toggle_favorite("snip-docker-prune").unwrap();
        assert!(fav_after);
        assert!(storage.snippets().get_by_id("snip-docker-prune").unwrap().unwrap().is_favorite);

        // 5. 新建与删除片段
        let new_snip = SnippetRecord::new("snip-test", "测试脚本", "echo 'hello'", "bash");
        storage.snippets().save(&new_snip).unwrap();
        assert_eq!(storage.snippets().list_all().unwrap().len(), 11);
        storage.snippets().delete("snip-test").unwrap();
        assert_eq!(storage.snippets().list_all().unwrap().len(), 10);

        // 6. 新建与移动分组
        let new_grp = SnippetGroupRecord::child("sgrp-test-child", "子分组", "sgrp-docker", 1);
        storage.snippets().save_group(&new_grp).unwrap();
        assert_eq!(storage.snippets().list_groups().unwrap().len(), 6);
        storage.snippets().delete_group("sgrp-test-child").unwrap();
        assert_eq!(storage.snippets().list_groups().unwrap().len(), 5);
    }

    #[test]
    fn test_mock_storage_tunnels_crud_and_status() {
        let storage = MockStorage::new_seeded();

        // 1. 种子加载校验
        let tunnels = storage.tunnels().list_all().unwrap();
        assert_eq!(tunnels.len(), 6);

        // 2. 按类型过滤
        let locals = storage.tunnels().list_by_type(TunnelType::Local).unwrap();
        assert_eq!(locals.len(), 2);
        let remotes = storage.tunnels().list_by_type(TunnelType::Remote).unwrap();
        assert_eq!(remotes.len(), 1);
        let proxies = storage.tunnels().list_by_type(TunnelType::ProxyServer).unwrap();
        assert_eq!(proxies.len(), 1);

        // 3. 搜索过滤
        let search_mysql = storage.tunnels().search("mysql").unwrap();
        assert_eq!(search_mysql.len(), 1);
        assert_eq!(search_mysql[0].id, "tun-mysql-prod");

        // 4. 启停状态切换
        let is_running = storage.tunnels().get_by_id("tun-webhook-dev").unwrap().unwrap().is_running;
        assert!(!is_running);
        storage.tunnels().set_running("tun-webhook-dev", true).unwrap();
        let is_running_after = storage.tunnels().get_by_id("tun-webhook-dev").unwrap().unwrap().is_running;
        assert!(is_running_after);

        // 5. 流量更新
        storage.tunnels().update_metrics("tun-webhook-dev", 2, 1024, 2048).unwrap();
        let updated = storage.tunnels().get_by_id("tun-webhook-dev").unwrap().unwrap();
        assert_eq!(updated.active_connections, 2);
        assert!(updated.total_bytes_in >= 1024);

        // 6. 新建与删除
        let mut new_tun = updated.clone();
        new_tun.id = "tun-temp-test".to_string();
        new_tun.name = "临时测试隧道".to_string();
        storage.tunnels().save(&new_tun).unwrap();
        assert_eq!(storage.tunnels().list_all().unwrap().len(), 7);

        let deleted = storage.tunnels().delete("tun-temp-test").unwrap();
        assert!(deleted);
        assert_eq!(storage.tunnels().list_all().unwrap().len(), 6);
    }

    #[test]
    fn test_mock_storage_config_crud_and_update() {
        let storage = MockStorage::new_seeded();

        // 1. 读取初始默认配置
        let cfg = storage.config().get().unwrap();
        assert_eq!(cfg.language, "zh-CN");
        assert_eq!(cfg.theme_id, "builtin.ui.darcula");
        assert_eq!(cfg.font_size, 13.0);
        assert!(!cfg.flag_desktop_notifications);

        // 2. 局部修改 update
        let updated = storage.config().update(Box::new(|c| {
            c.font_size = 15.0;
            c.theme_id = "builtin.ui.one-dark".to_string();
            c.flag_desktop_notifications = true;
        })).unwrap();
        assert_eq!(updated.font_size, 15.0);
        assert_eq!(updated.theme_id, "builtin.ui.one-dark");
        assert!(updated.flag_desktop_notifications);

        // 3. 读取验证
        let fresh = storage.config().get().unwrap();
        assert_eq!(fresh.font_size, 15.0);
        assert_eq!(fresh.theme_id, "builtin.ui.one-dark");

        // 4. 重置回默认值
        let reset = storage.config().reset_to_default().unwrap();
        assert_eq!(reset.font_size, 13.0);
        assert_eq!(reset.theme_id, "builtin.ui.darcula");
        assert!(!reset.flag_desktop_notifications);
    }
}



