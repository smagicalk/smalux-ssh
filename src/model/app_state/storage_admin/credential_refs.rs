use crate::core::CoreState;
use crate::model::{AuthProfile, CredentialKind, CredentialMetadata, SecretRef};

pub(crate) fn next_credential_copy_name(
    credentials: &[CredentialMetadata],
    base_name: &str,
) -> String {
    let mut candidate = format!("{base_name} 复制");
    let mut index = 2;
    while credentials
        .iter()
        .any(|credential| credential.name == candidate)
    {
        candidate = format!("{base_name} 复制 {index}");
        index += 1;
    }
    candidate
}

pub(crate) fn next_secret_ref(
    state: &CoreState,
    namespace: &str,
    fallback: &str,
    name: &str,
) -> SecretRef {
    let slug = secret_ref_slug(name);
    let base = if slug.is_empty() {
        fallback.to_owned()
    } else {
        slug
    };
    let mut candidate = format!("secret://{namespace}/{base}");
    let mut index = 2;
    while secret_ref_exists(state, &candidate) {
        candidate = format!("secret://{namespace}/{base}-{index}");
        index += 1;
    }
    SecretRef(candidate)
}

pub(crate) fn credential_secret_namespace(kind: &CredentialKind) -> (&'static str, &'static str) {
    match kind {
        CredentialKind::Password => ("passwords", "password"),
        CredentialKind::PrivateKey => ("keys", "private-key"),
        CredentialKind::Agent => ("agents", "agent"),
        CredentialKind::Certificate => ("certs", "certificate"),
    }
}

pub(crate) fn auth_profile_uses_secret_ref(auth: &AuthProfile, secret_ref: &SecretRef) -> bool {
    match auth {
        AuthProfile::Password { secret, .. } => secret == secret_ref,
        AuthProfile::Key {
            key, passphrase, ..
        } => key == secret_ref || passphrase.as_ref().is_some_and(|value| value == secret_ref),
        AuthProfile::Agent { .. } => false,
        AuthProfile::Certificate {
            key,
            passphrase,
            certificate,
            ..
        } => {
            key == secret_ref
                || certificate == secret_ref
                || passphrase.as_ref().is_some_and(|value| value == secret_ref)
        }
    }
}

fn secret_ref_slug(name: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for character in name.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            last_dash = false;
        } else if !last_dash && !slug.is_empty() {
            slug.push('-');
            last_dash = true;
        }
    }
    if slug.ends_with('-') {
        slug.pop();
    }
    slug
}

fn secret_ref_exists(state: &CoreState, candidate: &str) -> bool {
    state
        .storage
        .secrets
        .iter()
        .any(|secret| secret.secret_ref.0 == candidate)
        || state.storage.credentials.iter().any(|credential| {
            credential
                .secret
                .as_ref()
                .is_some_and(|reference| reference.0 == candidate)
        })
}
