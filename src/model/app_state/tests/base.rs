use super::*;

#[test]
fn default_state_starts_empty() {
    let state = AppState::default();

    assert_eq!(state.config.app_name, "smagicalssh");
    assert_eq!(state.sessions.active_count(), 0);
    assert_eq!(state.storage.host_count(), 0);
    assert_eq!(state.terminal.tab_count(), 0);
    assert_eq!(state.backend_commands.pending_count(), 0);
}

#[test]
fn backend_event_message_updates_existing_session_state() {
    let mut state = AppState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenShell { host_id });
    state.backend_commands.drain();
    let session_id = state.sessions.tabs[0].id;

    let outcome = state.apply(Message::BackendEventReceived(BackendEvent::Connected {
        session_id,
    }));

    assert!(outcome.changed());
    assert_eq!(outcome.applied_backend_events, 1);
    assert!(matches!(
        state.sessions.tabs[0].status,
        SessionStatus::Connected
    ));
}

#[test]
fn remove_credential_message_updates_storage() {
    let mut state = AppState::default();
    state
        .storage
        .upsert_credential(crate::model::CredentialMetadata {
            id: crate::model::CredentialId(uuid::Uuid::new_v4()),
            name: "deploy".to_owned(),
            kind: crate::model::CredentialKind::Password,
            group_id: None,
            username: Some("deploy".to_owned()),
            secret: Some(crate::model::SecretRef("password:deploy".to_owned())),
            key_algorithm: None,
            fingerprint: None,
        });

    let outcome = state.apply(Message::RemoveCredential {
        name: "deploy".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(state.storage.credential_count(), 0);
}

#[test]
fn update_credential_metadata_message_updates_storage() {
    let mut state = AppState::default();
    state
        .storage
        .upsert_credential(crate::model::CredentialMetadata {
            id: crate::model::CredentialId(uuid::Uuid::new_v4()),
            name: "deploy".to_owned(),
            kind: crate::model::CredentialKind::PrivateKey,
            group_id: None,
            username: Some("deploy".to_owned()),
            secret: Some(crate::model::SecretRef("secret://keys/deploy".to_owned())),
            key_algorithm: Some(crate::model::KeyAlgorithm::Ed25519),
            fingerprint: None,
        });

    let outcome = state.apply(Message::UpdateCredentialMetadata {
        original_name: "deploy".to_owned(),
        name: "deploy-prod".to_owned(),
        group_id: None,
        algorithm: Some(crate::model::KeyAlgorithm::Rsa),
    });

    assert!(outcome.changed());
    assert_eq!(state.storage.credentials[0].name, "deploy-prod");
    assert_eq!(
        state.storage.credentials[0].key_algorithm,
        Some(crate::model::KeyAlgorithm::Rsa)
    );
}

#[test]
fn update_credential_secret_message_updates_storage() {
    let mut state = AppState::default();
    let secret_ref = crate::model::SecretRef("secret://passwords/deploy".to_owned());
    state
        .storage
        .upsert_secret(crate::model::SecretRecord::local_plaintext(
            secret_ref.clone(),
            crate::model::SecretMaterialKind::Password,
            b"old-password".to_vec(),
        ));
    state
        .storage
        .upsert_credential(crate::model::CredentialMetadata {
            id: crate::model::CredentialId(uuid::Uuid::new_v4()),
            name: "deploy".to_owned(),
            kind: crate::model::CredentialKind::Password,
            group_id: None,
            username: Some("deploy".to_owned()),
            secret: Some(secret_ref),
            key_algorithm: None,
            fingerprint: None,
        });

    let outcome = state.apply(Message::UpdateCredentialSecret {
        name: "deploy".to_owned(),
        secret_text: "new-password".to_owned(),
    });

    assert!(outcome.changed());
    assert_eq!(
        state.storage.secrets[0].encrypted_payload.as_deref(),
        Some(b"new-password".as_slice())
    );
}

#[test]
fn trust_known_host_message_marks_entry_trusted() {
    let mut state = AppState::default();
    state
        .storage
        .upsert_known_host(crate::model::KnownHostEntry {
            host: "example.com".to_owned(),
            port: 22,
            key_algorithm: crate::model::KeyAlgorithm::Ed25519,
            fingerprint: "SHA256:demo".to_owned(),
            trusted: false,
        });

    let outcome = state.apply(Message::TrustKnownHost {
        host: "example.com".to_owned(),
        port: 22,
    });

    assert!(outcome.changed());
    assert!(state.storage.known_hosts[0].trusted);
}

#[test]
fn remove_known_host_message_deletes_entry() {
    let mut state = AppState::default();
    state
        .storage
        .upsert_known_host(crate::model::KnownHostEntry::untrusted(
            "example.com",
            22,
            crate::model::KeyAlgorithm::Ed25519,
            "SHA256:demo",
        ));

    let outcome = state.apply(Message::RemoveKnownHost {
        host: "example.com".to_owned(),
        port: 22,
    });

    assert!(outcome.changed());
    assert_eq!(state.storage.known_host_count(), 0);
}
