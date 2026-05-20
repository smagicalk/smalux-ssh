//! 凭据元数据的内存索引操作。

use smagical_core::CredentialMetadata;

use super::StorageManager;

impl StorageManager {
    /// 保存或更新凭据元数据。
    pub fn upsert_credential(&mut self, credential: CredentialMetadata) {
        if let Some(existing) = self
            .credentials
            .iter_mut()
            .find(|existing| existing.name == credential.name)
        {
            *existing = credential;
        } else {
            self.credentials.push(credential);
        }
    }

    /// 删除凭据元数据。
    pub fn remove_credential(&mut self, name: &str) -> bool {
        let before = self.credentials.len();
        self.credentials
            .retain(|credential| credential.name != name);
        before != self.credentials.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smagical_core::{CredentialKind, KeyAlgorithm, SecretRef};

    #[test]
    fn credentials_can_be_upserted_and_removed_by_name() {
        let mut storage = StorageManager::default();

        storage.upsert_credential(CredentialMetadata {
            name: "deploy".to_owned(),
            kind: CredentialKind::Password,
            username: Some("deploy".to_owned()),
            secret: Some(SecretRef("password:deploy".to_owned())),
            key_algorithm: None,
            fingerprint: None,
        });
        storage.upsert_credential(CredentialMetadata {
            name: "deploy".to_owned(),
            kind: CredentialKind::PrivateKey,
            username: Some("deploy".to_owned()),
            secret: Some(SecretRef("key:deploy".to_owned())),
            key_algorithm: Some(KeyAlgorithm::Ed25519),
            fingerprint: Some("SHA256:key".to_owned()),
        });

        assert_eq!(storage.credential_count(), 1);
        assert!(matches!(
            storage.credentials[0].kind,
            CredentialKind::PrivateKey
        ));
        assert!(storage.remove_credential("deploy"));
        assert!(!storage.remove_credential("deploy"));
    }
}
