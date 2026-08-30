use std::sync::{Arc, RwLock};
use crate::domain::{
    group::GroupRecord,
    history::HistoryRecord,
    host::{HostRecord, HostStatus},
};
use super::{
    AppStorage, GroupRepository, HistoryRepository, HostRepository, StorageError, StorageResult,
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


/// 聚合内存存储实现 (MockStorage)
#[derive(Debug, Clone)]
pub struct MockStorage {
    hosts_repo: MockHostRepository,
    groups_repo: MockGroupRepository,
    history_repo: MockHistoryRepository,
}

impl Default for MockStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl MockStorage {
    /// 创建空的 MockStorage
    pub fn new() -> Self {
        Self {
            hosts_repo: MockHostRepository::new(),
            groups_repo: MockGroupRepository::new(),
            history_repo: MockHistoryRepository::new(),
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

        tracing::info!(
            target: "smagical_core::storage",
            "初始化 MockStorage 预设种子引擎完成: 已注入 {} 个层级分组, {} 台主机资产, {} 条历史会话记录 (含 {} 份屏幕快照)",
            groups.len(),
            hosts.len(),
            history.len(),
            snapshots.len()
        );

        Self {
            hosts_repo: MockHostRepository::with_hosts(hosts),
            groups_repo: MockGroupRepository::with_groups(groups),
            history_repo: MockHistoryRepository::with_history_and_snapshots(history, snapshots),
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
}



