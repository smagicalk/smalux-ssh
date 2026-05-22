//! 快速新增主机认证草稿转换。

use crate::model::{AuthProfile, SecretRef};

use super::{QuickHostAuthDraft, QuickHostAuthKind, QuickHostDraftError};

pub(super) fn quick_host_auth_kind_label(kind: QuickHostAuthKind) -> &'static str {
    match kind {
        QuickHostAuthKind::Password => "Password",
        QuickHostAuthKind::Key => "Key",
        QuickHostAuthKind::Agent => "ssh-agent",
        QuickHostAuthKind::Certificate => "Certificate",
    }
}

pub(super) fn build_quick_host_auth(
    draft: &QuickHostAuthDraft,
    username: &str,
) -> Result<AuthProfile, QuickHostDraftError> {
    match draft.kind {
        QuickHostAuthKind::Password => password_auth(draft, username),
        QuickHostAuthKind::Key => key_auth(draft, username),
        QuickHostAuthKind::Agent => agent_auth(draft, username),
        QuickHostAuthKind::Certificate => certificate_auth(draft, username),
    }
}

fn password_auth(
    draft: &QuickHostAuthDraft,
    username: &str,
) -> Result<AuthProfile, QuickHostDraftError> {
    let secret_ref = draft.password_secret_ref.trim();
    if secret_ref.is_empty() {
        return Err(QuickHostDraftError::MissingPasswordSecretRef);
    }

    Ok(AuthProfile::Password {
        username: username.to_owned(),
        secret: SecretRef(secret_ref.to_owned()),
    })
}

fn key_auth(
    draft: &QuickHostAuthDraft,
    username: &str,
) -> Result<AuthProfile, QuickHostDraftError> {
    let private_key_ref = draft.private_key_ref.trim();
    if private_key_ref.is_empty() {
        return Err(QuickHostDraftError::MissingPrivateKeyRef);
    }

    let passphrase_ref = draft.passphrase_ref.trim();

    Ok(AuthProfile::Key {
        username: username.to_owned(),
        key: SecretRef(private_key_ref.to_owned()),
        passphrase: if passphrase_ref.is_empty() {
            None
        } else {
            Some(SecretRef(passphrase_ref.to_owned()))
        },
    })
}

fn agent_auth(
    draft: &QuickHostAuthDraft,
    username: &str,
) -> Result<AuthProfile, QuickHostDraftError> {
    let key_hint = draft.key_hint.trim();

    Ok(AuthProfile::Agent {
        username: username.to_owned(),
        key_hint: if key_hint.is_empty() {
            None
        } else {
            Some(key_hint.to_owned())
        },
    })
}

fn certificate_auth(
    draft: &QuickHostAuthDraft,
    username: &str,
) -> Result<AuthProfile, QuickHostDraftError> {
    let private_key_ref = draft.private_key_ref.trim();
    if private_key_ref.is_empty() {
        return Err(QuickHostDraftError::MissingPrivateKeyRef);
    }

    let certificate_ref = draft.certificate_ref.trim();
    if certificate_ref.is_empty() {
        return Err(QuickHostDraftError::MissingCertificateRef);
    }

    Ok(AuthProfile::Certificate {
        username: username.to_owned(),
        key: SecretRef(private_key_ref.to_owned()),
        passphrase: if draft.passphrase_ref.trim().is_empty() {
            None
        } else {
            Some(SecretRef(draft.passphrase_ref.trim().to_owned()))
        },
        certificate: SecretRef(certificate_ref.to_owned()),
    })
}
