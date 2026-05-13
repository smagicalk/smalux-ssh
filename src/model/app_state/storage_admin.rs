//! 本地安全资产的管理操作。
//!
//! 这里专门处理凭据元数据和 Known Hosts 的增删改，避免把存储管理逻辑继续塞进
//! `app_state.rs` 主文件。

use super::{AppState, AppUpdateOutcome};

impl AppState {
    /// 删除一个已保存的凭据元数据。
    pub(super) fn remove_credential(&mut self, name: &str) -> AppUpdateOutcome {
        if self.storage.remove_credential(name) {
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            AppUpdateOutcome {
                error: Some(format!("找不到凭据：{name}")),
                ..AppUpdateOutcome::default()
            }
        }
    }

    /// 将指定 Known Hosts 记录标记为可信。
    pub(super) fn trust_known_host(&mut self, host: &str, port: u16) -> AppUpdateOutcome {
        if let Some(entry) = self
            .storage
            .known_hosts
            .iter_mut()
            .find(|entry| entry.host == host && entry.port == port)
        {
            if entry.trusted {
                return AppUpdateOutcome::default();
            }

            entry.trusted = true;
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            AppUpdateOutcome {
                error: Some(format!("找不到 Known Hosts 记录：{host}:{port}")),
                ..AppUpdateOutcome::default()
            }
        }
    }

    /// 删除一个 Known Hosts 记录。
    pub(super) fn remove_known_host(&mut self, host: &str, port: u16) -> AppUpdateOutcome {
        if self.storage.remove_known_host(host, port) {
            AppUpdateOutcome {
                state_changed: true,
                ..AppUpdateOutcome::default()
            }
        } else {
            AppUpdateOutcome {
                error: Some(format!("找不到 Known Hosts 记录：{host}:{port}")),
                ..AppUpdateOutcome::default()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        CredentialKind, CredentialMetadata, KeyAlgorithm, KnownHostEntry, SecretRef,
    };

    #[test]
    fn remove_credential_reports_state_change_and_failure() {
        let mut state = AppState::default();
        state.storage.upsert_credential(CredentialMetadata {
            name: "deploy".to_owned(),
            kind: CredentialKind::Password,
            username: Some("deploy".to_owned()),
            secret: Some(SecretRef("password:deploy".to_owned())),
            key_algorithm: None,
            fingerprint: None,
        });

        let outcome = state.remove_credential("deploy");

        assert!(outcome.changed());
        assert_eq!(state.storage.credential_count(), 0);
        assert!(state.remove_credential("missing").error.is_some());
    }

    #[test]
    fn known_host_can_be_trusted_and_removed() {
        let mut state = AppState::default();
        state.storage.upsert_known_host(KnownHostEntry {
            host: "example.com".to_owned(),
            port: 22,
            key_algorithm: KeyAlgorithm::Ed25519,
            fingerprint: "SHA256:demo".to_owned(),
            trusted: false,
        });

        let trust_outcome = state.trust_known_host("example.com", 22);

        assert!(trust_outcome.changed());
        assert!(state.storage.known_hosts[0].trusted);
        assert!(state.remove_known_host("example.com", 22).changed());
        assert!(state.remove_known_host("example.com", 22).error.is_some());
    }
}
