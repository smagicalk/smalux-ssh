//! 系统凭据库适配。

use crate::model::SecretRef;

use super::{SecretStore, SecurityError};

/// 基于系统 keyring 的凭据存储。
#[derive(Debug, Clone)]
pub struct KeyringSecretStore {
    service: String,
}

impl KeyringSecretStore {
    /// 创建指定服务名的系统凭据库存储。
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    fn entry(&self, reference: &SecretRef) -> Result<keyring::Entry, SecurityError> {
        keyring::Entry::new(&self.service, &reference.0).map_err(SecurityError::store)
    }
}

impl Default for KeyringSecretStore {
    fn default() -> Self {
        Self::new("smagicalssh")
    }
}

impl SecretStore for KeyringSecretStore {
    fn get_secret(&self, reference: &SecretRef) -> Result<String, SecurityError> {
        self.entry(reference)?
            .get_password()
            .map_err(|error| match error {
                keyring::Error::NoEntry => SecurityError::MissingSecret(reference.clone()),
                other => SecurityError::store(other),
            })
    }

    fn set_secret(&mut self, reference: &SecretRef, value: &str) -> Result<(), SecurityError> {
        self.entry(reference)?
            .set_password(value)
            .map_err(SecurityError::store)
    }

    fn delete_secret(&mut self, reference: &SecretRef) -> Result<bool, SecurityError> {
        match self.entry(reference)?.delete_credential() {
            Ok(()) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(error) => Err(SecurityError::store(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyring_secret_store_has_default_service_name() {
        let store = KeyringSecretStore::default();

        assert_eq!(store.service, "smagicalssh");
    }
}
