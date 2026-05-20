//! Known Hosts 记录的内存索引和校验操作。

use smagical_core::{HostKeyVerification, KnownHostEntry};

use super::StorageManager;

impl StorageManager {
    /// 保存或更新 Known Hosts 记录。
    pub fn upsert_known_host(&mut self, entry: KnownHostEntry) {
        if let Some(existing) = self
            .known_hosts
            .iter_mut()
            .find(|existing| existing.host == entry.host && existing.port == entry.port)
        {
            *existing = entry;
        } else {
            self.known_hosts.push(entry);
        }
    }

    /// 校验远端主机密钥指纹。
    pub fn verify_host_key(&self, host: &str, port: u16, fingerprint: &str) -> HostKeyVerification {
        self.known_hosts
            .iter()
            .find(|entry| entry.host == host && entry.port == port)
            .map(|entry| entry.verify(host, port, fingerprint))
            .unwrap_or(HostKeyVerification::Unknown)
    }

    /// 删除 Known Hosts 记录。
    pub fn remove_known_host(&mut self, host: &str, port: u16) -> bool {
        let before = self.known_hosts.len();
        self.known_hosts
            .retain(|entry| entry.host != host || entry.port != port);
        before != self.known_hosts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smagical_core::KeyAlgorithm;

    #[test]
    fn known_hosts_can_be_upserted_verified_and_removed() {
        let mut storage = StorageManager::default();

        storage.upsert_known_host(KnownHostEntry {
            host: "example.com".to_owned(),
            port: 22,
            key_algorithm: KeyAlgorithm::Ed25519,
            fingerprint: "SHA256:old".to_owned(),
            trusted: true,
        });
        storage.upsert_known_host(KnownHostEntry {
            host: "example.com".to_owned(),
            port: 22,
            key_algorithm: KeyAlgorithm::Ed25519,
            fingerprint: "SHA256:new".to_owned(),
            trusted: true,
        });

        assert_eq!(storage.known_host_count(), 1);
        assert_eq!(
            storage.verify_host_key("example.com", 22, "SHA256:new"),
            HostKeyVerification::Trusted
        );
        assert_eq!(
            storage.verify_host_key("example.com", 22, "SHA256:old"),
            HostKeyVerification::Mismatch {
                expected: "SHA256:new".to_owned(),
                actual: "SHA256:old".to_owned(),
            }
        );
        assert_eq!(
            storage.verify_host_key("missing.example.com", 22, "SHA256:new"),
            HostKeyVerification::Unknown
        );
        assert!(storage.remove_known_host("example.com", 22));
        assert!(!storage.remove_known_host("example.com", 22));
    }
}
