//! 后端泵中的主机密钥拒绝记录。

use crate::backend::BackendExecutionError;
use crate::model::{HostKeyVerification, KnownHostEntry};

use super::super::AppState;

impl AppState {
    pub(super) fn record_rejected_host_key(&mut self, error: &BackendExecutionError) -> bool {
        let BackendExecutionError::HostKeyRejected {
            host,
            port,
            key_algorithm,
            fingerprint,
            verification,
        } = error
        else {
            return false;
        };
        if matches!(verification, HostKeyVerification::Mismatch { .. }) {
            return false;
        }

        self.storage.upsert_known_host(KnownHostEntry::untrusted(
            host.clone(),
            *port,
            key_algorithm.clone(),
            fingerprint.clone(),
        ));
        true
    }
}
