//! 后端泵中的主机密钥拒绝记录。
//!
//! 后端只负责告诉状态层主机密钥被拒绝，状态层决定是否把它记入 Known Hosts。
//! 未知密钥会被保存为“不信任”，由用户在 UI 中确认；密钥不匹配属于高风险情况，
//! 不能自动覆盖旧记录。

use crate::backend::BackendExecutionError;
use crate::core::CoreState;
use crate::model::{HostKeyVerification, KnownHostEntry};

impl CoreState {
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
        // mismatch 说明已有记录和当前服务端密钥冲突，不能悄悄把新密钥写入本地。
        if matches!(verification, HostKeyVerification::Mismatch { .. }) {
            return false;
        }

        // unknown 场景保存为 untrusted，UI 可以展示指纹并等待用户显式信任。
        self.storage.upsert_known_host(KnownHostEntry::untrusted(
            host.clone(),
            *port,
            key_algorithm.clone(),
            fingerprint.clone(),
        ));
        true
    }
}
