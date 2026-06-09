//! 凭据元数据的内存索引操作。

use smagical_core::{
    CredentialGroup, CredentialGroupId, CredentialId, CredentialInspection, CredentialMetadata,
    SecretRecord, SecretRef,
};

use super::StorageManager;

impl StorageManager {
    /// 保存或更新密钥分组。
    pub fn upsert_credential_group(&mut self, group: CredentialGroup) {
        if let Some(existing) = self
            .credential_groups
            .iter_mut()
            .find(|existing| existing.id == group.id)
        {
            *existing = group;
        } else {
            self.credential_groups.push(group);
        }
    }

    /// 判断密钥分组是否存在。
    pub fn credential_group_exists(&self, group_id: CredentialGroupId) -> bool {
        self.credential_groups
            .iter()
            .any(|group| group.id == group_id)
    }

    /// 判断密钥分组是否包含子分组。
    pub fn credential_group_has_children(&self, group_id: CredentialGroupId) -> bool {
        self.credential_groups
            .iter()
            .any(|group| group.parent_id == Some(group_id))
    }

    /// 修改密钥分组名称。
    pub fn rename_credential_group(&mut self, group_id: CredentialGroupId, name: String) -> bool {
        if let Some(group) = self
            .credential_groups
            .iter_mut()
            .find(|group| group.id == group_id)
        {
            group.name = name;
            true
        } else {
            false
        }
    }

    /// 删除密钥分组。
    pub fn remove_credential_group(&mut self, group_id: CredentialGroupId) -> bool {
        let before = self.credential_groups.len();
        self.credential_groups.retain(|group| group.id != group_id);
        before != self.credential_groups.len()
    }

    /// 保存或更新凭据元数据。
    pub fn upsert_credential(&mut self, credential: CredentialMetadata) {
        if let Some(existing) = self
            .credentials
            .iter_mut()
            .find(|existing| existing.id == credential.id || existing.name == credential.name)
        {
            *existing = credential;
        } else {
            self.credentials.push(credential);
        }
    }

    /// 保存或更新凭据内容解析缓存。
    pub fn upsert_credential_inspection(&mut self, inspection: CredentialInspection) {
        if let Some(existing) = self
            .credential_inspections
            .iter_mut()
            .find(|existing| existing.credential_id == inspection.credential_id)
        {
            *existing = inspection;
        } else {
            self.credential_inspections.push(inspection);
        }
    }

    /// 删除凭据内容解析缓存。
    pub fn remove_credential_inspection(&mut self, credential_id: CredentialId) -> bool {
        let before = self.credential_inspections.len();
        self.credential_inspections
            .retain(|inspection| inspection.credential_id != credential_id);
        before != self.credential_inspections.len()
    }

    /// 保存或更新安全存储记录。
    pub fn upsert_secret(&mut self, secret: SecretRecord) {
        if let Some(existing) = self
            .secrets
            .iter_mut()
            .find(|existing| existing.secret_ref == secret.secret_ref)
        {
            *existing = secret;
        } else {
            self.secrets.push(secret);
        }
    }

    /// 删除安全存储记录。
    pub fn remove_secret(&mut self, secret_ref: &SecretRef) -> bool {
        let before = self.secrets.len();
        self.secrets
            .retain(|secret| &secret.secret_ref != secret_ref);
        before != self.secrets.len()
    }

    /// 删除凭据元数据。
    pub fn remove_credential(&mut self, name: &str) -> bool {
        let removed_ids = self
            .credentials
            .iter()
            .filter(|credential| credential.name == name)
            .map(|credential| credential.id)
            .collect::<Vec<_>>();
        let before = self.credentials.len();
        self.credentials
            .retain(|credential| credential.name != name);
        for credential_id in removed_ids {
            self.remove_credential_inspection(credential_id);
        }
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
            id: smagical_core::CredentialId(uuid::Uuid::new_v4()),
            name: "deploy".to_owned(),
            kind: CredentialKind::Password,
            group_id: None,
            username: Some("deploy".to_owned()),
            secret: Some(SecretRef("password:deploy".to_owned())),
            key_algorithm: None,
            fingerprint: None,
        });
        storage.upsert_credential(CredentialMetadata {
            id: smagical_core::CredentialId(uuid::Uuid::new_v4()),
            name: "deploy".to_owned(),
            kind: CredentialKind::PrivateKey,
            group_id: None,
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
