//! 内存网络隧道与代理仓储实现

use std::sync::{Arc, RwLock};
use smagical_core::domain::tunnel::{TunnelRecord, TunnelType};
use smagical_core::storage::{StorageError, StorageResult, TunnelRepository};

/// 线程安全的内存网络隧道仓储实现
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
