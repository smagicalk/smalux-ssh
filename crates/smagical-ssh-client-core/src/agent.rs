//! SSH agent 身份选择。

use russh::keys::PublicKey;

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

fn agent_identity_matches(identity: &PublicKey, hint: &str) -> bool {
    let fingerprint = host_key_fingerprint(identity);
    identity.comment().contains(hint)
        || fingerprint == hint
        || format!("{:?}", identity.algorithm()).contains(hint)
}
