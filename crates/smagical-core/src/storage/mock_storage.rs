use std::sync::{Arc, RwLock};
use crate::domain::{group::GroupRecord, host::HostRecord};
use super::{AppStorage, GroupRepository, HostRepository, StorageError, StorageResult};

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
        let new_level = match &new_parent_str {
            Some(p_id) => {
                let parent = write_guard.iter().find(|g| &g.id == p_id)
                    .ok_or_else(|| StorageError::NotFound(format!("目标父级分组不存在: {}", p_id)))?;
                parent.level + 1
            }
            None => 0,
        };

        if let Some(g) = write_guard.iter_mut().find(|g| g.id == id) {
            g.parent_id = new_parent_str.clone();
            g.level = new_level;
            tracing::info!(target: "smagical_core::storage", "MockStorage 迁移分组: {} -> 上级: {:?}", id, new_parent_str);
            Ok(())
        } else {
            Err(StorageError::NotFound(format!("分组不存在: {}", id)))
        }
    }
}

/// 聚合内存存储实现 (MockStorage)
#[derive(Debug, Clone)]
pub struct MockStorage {
    hosts_repo: MockHostRepository,
    groups_repo: MockGroupRepository,
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
                status: "online".to_string(),
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
                status: "warning".to_string(),
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
                status: "online".to_string(),
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
                status: "online".to_string(),
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
                status: "online".to_string(),
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
                status: "online".to_string(),
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
                status: "online".to_string(),
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
                status: "online".to_string(),
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
                status: "offline".to_string(),
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
                status: "offline".to_string(),
                ping_ms: 0,
                sort_order: 9,
                notes: "预发接口测试机".to_string(),
            },
        ];

        tracing::info!(
            target: "smagical_core::storage",
            "初始化 MockStorage 预设种子引擎完成: 已注入 {} 个层级分组, {} 台主机资产",
            groups.len(),
            hosts.len()
        );

        Self {
            hosts_repo: MockHostRepository::with_hosts(hosts),
            groups_repo: MockGroupRepository::with_groups(groups),
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
}
