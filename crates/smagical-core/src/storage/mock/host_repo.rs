//! 内存主机仓储实现

use std::sync::{Arc, RwLock};
use crate::domain::host::HostRecord;
use crate::storage::{HostRepository, StorageError, StorageResult};

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
