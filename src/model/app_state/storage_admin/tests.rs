use super::super::*;
use crate::model::{CredentialKind, CredentialMetadata, KeyAlgorithm, KnownHostEntry, SecretRef};

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
