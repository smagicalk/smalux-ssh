//! 内存安全凭据仓储实现

use std::sync::{Arc, RwLock};
use smagical_core::domain::{credential::{CredentialRecord, CredentialType}, host::HostRecord};
use smagical_core::storage::{CredentialRepository, StorageError, StorageResult};

/// 线程安全的内存安全凭据仓储实现
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

#[async_trait::async_trait]
impl CredentialRepository for MockCredentialRepository {
    async fn list_all(&self) -> StorageResult<Vec<CredentialRecord>> {
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

    async fn list_by_type(&self, cred_type: CredentialType) -> StorageResult<Vec<CredentialRecord>> {
        let all = self.list_all().await?;
        Ok(all.into_iter().filter(|c| c.cred_type == cred_type).collect())
    }

    async fn get_by_id(&self, id: &str) -> StorageResult<Option<CredentialRecord>> {
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

    async fn search(&self, query: &str) -> StorageResult<Vec<CredentialRecord>> {
        let all = self.list_all().await?;
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

    async fn get_bound_hosts(&self, id: &str) -> StorageResult<Vec<String>> {
        if let Some(ref hosts_lock) = self.hosts {
            let h_list = hosts_lock.read().map_err(|e| StorageError::Backend(e.to_string()))?;
            Ok(h_list.iter().filter(|h| h.credential_id.as_deref() == Some(id)).map(|h| h.id.clone()).collect())
        } else {
            Ok(Vec::new())
        }
    }

    async fn save(&self, record: &CredentialRecord) -> StorageResult<()> {
        let mut write_guard = self.credentials.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(pos) = write_guard.iter().position(|c| c.id == record.id) {
            write_guard[pos] = record.clone();
        } else {
            write_guard.push(record.clone());
        }
        tracing::debug!(target: "smagical_storage::mock", "MockStorage 保存凭据: {} ({})", record.name, record.algorithm);
        Ok(())
    }

    async fn save_batch(&self, records: &[CredentialRecord]) -> StorageResult<()> {
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

    async fn delete(&self, id: &str) -> StorageResult<bool> {
        let mut write_guard = self.credentials.write().map_err(|e| StorageError::Backend(e.to_string()))?;
        if let Some(pos) = write_guard.iter().position(|c| c.id == id) {
            write_guard.remove(pos);
            tracing::debug!(target: "smagical_storage::mock", "MockStorage 删除凭据: ID={}", id);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
