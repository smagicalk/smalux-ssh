//! 凭据存储抽象和测试用内存实现。

use std::collections::HashMap;

use crate::model::SecretRef;

use super::SecurityError;

/// 凭据存储接口。
///
/// 生产环境使用系统凭据库实现，测试和导入流程可以使用内存实现。
pub trait SecretStore {
    fn get_secret(&self, reference: &SecretRef) -> Result<String, SecurityError>;
    fn set_secret(&mut self, reference: &SecretRef, value: &str) -> Result<(), SecurityError>;
    fn delete_secret(&mut self, reference: &SecretRef) -> Result<bool, SecurityError>;
}

/// 测试和导入流程使用的内存凭据存储。
#[derive(Debug, Clone, Default)]
pub struct MemorySecretStore {
    secrets: HashMap<SecretRef, String>,
}

impl MemorySecretStore {
    /// 创建空的内存凭据存储。
    pub fn new() -> Self {
        Self::default()
    }

    /// 返回当前保存的凭据数量，便于测试和导入统计。
    pub fn len(&self) -> usize {
        self.secrets.len()
    }

    /// 判断当前是否没有保存任何凭据。
    pub fn is_empty(&self) -> bool {
        self.secrets.is_empty()
    }
}

impl SecretStore for MemorySecretStore {
    fn get_secret(&self, reference: &SecretRef) -> Result<String, SecurityError> {
        self.secrets
            .get(reference)
            .cloned()
            .ok_or_else(|| SecurityError::MissingSecret(reference.clone()))
    }

    fn set_secret(&mut self, reference: &SecretRef, value: &str) -> Result<(), SecurityError> {
        self.secrets.insert(reference.clone(), value.to_owned());
        Ok(())
    }

    fn delete_secret(&mut self, reference: &SecretRef) -> Result<bool, SecurityError> {
        Ok(self.secrets.remove(reference).is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_secret_store_round_trips_and_deletes_secret() {
        let mut store = MemorySecretStore::new();
        let reference = SecretRef("password:root".to_owned());

        assert!(store.is_empty());
        store
            .set_secret(&reference, "secret")
            .expect("内存凭据应该可以写入");

        assert_eq!(store.len(), 1);
        assert_eq!(store.get_secret(&reference), Ok("secret".to_owned()));
        assert_eq!(store.delete_secret(&reference), Ok(true));
        assert_eq!(store.delete_secret(&reference), Ok(false));
        assert_eq!(
            store.get_secret(&reference),
            Err(SecurityError::MissingSecret(reference))
        );
    }
}
