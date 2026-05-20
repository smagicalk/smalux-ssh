//! SSH agent 身份选择。

use russh::keys::{PrivateKey, PublicKey};
use smagical_backend_core::BackendExecutionError;

use crate::host_key::host_key_fingerprint;

/// 根据可选 hint 从 ssh-agent 身份列表中选择公钥。
pub fn select_agent_identity(
    identities: &[PublicKey],
    key_hint: Option<&str>,
) -> Option<PublicKey> {
    match key_hint {
        Some(hint) => identities
            .iter()
            .find(|identity| agent_identity_matches(identity, hint))
            .cloned(),
        None => identities.first().cloned(),
    }
}

/// 解码 OpenSSH 私钥并将失败映射成认证错误。
pub fn decode_private_key(
    private_key: &str,
    passphrase: Option<&str>,
    username: &str,
) -> Result<PrivateKey, BackendExecutionError> {
    russh::keys::decode_secret_key(private_key, passphrase)
        .map_err(|error| authentication_error(username, error))
}

/// 创建 ssh-agent 身份选择失败原因。
pub fn agent_identity_error(key_hint: Option<&str>) -> String {
    match key_hint {
        Some(hint) => format!("ssh-agent 中没有匹配的身份：{hint}"),
        None => "ssh-agent 中没有可用身份".to_owned(),
    }
}

/// 将认证错误转换成后端执行错误。
pub fn authentication_error(
    username: &str,
    error: impl std::error::Error,
) -> BackendExecutionError {
    BackendExecutionError::AuthenticationFailed {
        username: username.to_owned(),
        reason: error.to_string(),
    }
}

/// 创建服务端拒绝认证时的后端执行错误。
pub fn authentication_rejected_error(username: &str, method: &str) -> BackendExecutionError {
    BackendExecutionError::AuthenticationFailed {
        username: username.to_owned(),
        reason: format!("{method} 认证被服务器拒绝"),
    }
}

fn agent_identity_matches(identity: &PublicKey, hint: &str) -> bool {
    let fingerprint = host_key_fingerprint(identity);
    identity.comment().contains(hint)
        || fingerprint == hint
        || format!("{:?}", identity.algorithm()).contains(hint)
}
