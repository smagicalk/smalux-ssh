use crate::core::CoreState;
use crate::model::{
    AuthProfile, CredentialGroupId, CredentialKind, CredentialMetadata, Host, HostNetworkSelection,
    KeyAlgorithm, KnownHostEntry, Message, SecretMaterialKind, SecretRecord, SecretRef,
};
use russh::keys::{
    Algorithm, PrivateKey,
    ssh_key::{Certificate, LineEnding, certificate, rand_core::OsRng},
};
use std::fs;

fn credential_secret_ref(state: &CoreState, name: &str) -> String {
    state
        .storage
        .credentials
        .iter()
        .find(|credential| credential.name == name)
        .and_then(|credential| credential.secret.as_ref())
        .expect("credential should have secret ref")
        .0
        .clone()
}

fn private_key_fixture_payload() -> String {
    let mut rng = OsRng;
    PrivateKey::random(&mut rng, Algorithm::Ed25519)
        .expect("private key fixture should generate")
        .to_openssh(LineEnding::LF)
        .expect("private key fixture should encode")
        .to_string()
}

fn certificate_fixture_payload() -> String {
    let mut rng = OsRng;
    let ca_private_key = PrivateKey::random(&mut rng, Algorithm::Ed25519)
        .expect("CA private key fixture should generate");
    let subject_private_key = PrivateKey::random(&mut rng, Algorithm::Ed25519)
        .expect("subject private key fixture should generate");
    let mut builder = certificate::Builder::new_with_random_nonce(
        &mut rng,
        subject_private_key.public_key(),
        1,
        4_102_444_800,
    )
    .expect("certificate fixture builder should create");
    builder
        .serial(42)
        .expect("certificate fixture serial should set");
    builder
        .cert_type(certificate::CertType::User)
        .expect("certificate fixture type should set");
    builder
        .key_id("fixture-cert")
        .expect("certificate fixture key id should set");
    builder
        .valid_principal("deploy")
        .expect("certificate fixture principal should set");
    let certificate = builder
        .sign(&ca_private_key)
        .expect("certificate fixture should sign");
    let mut payload = certificate
        .to_openssh()
        .expect("certificate fixture should encode");
    if !payload.ends_with('\n') {
        payload.push('\n');
    }
    payload
}

#[test]
fn create_credential_group_adds_root_group() {
    let mut state = CoreState::default();

    let outcome =
        state.create_credential_group("生产密钥".to_owned(), CredentialKind::PrivateKey, None);

    assert!(outcome.changed());
    assert_eq!(state.storage.credential_group_count(), 1);
    assert_eq!(state.storage.credential_groups[0].name, "生产密钥");
    assert!(state.storage.credential_groups[0].parent_id.is_none());
}

#[test]
fn create_credential_group_can_use_existing_parent() {
    let mut state = CoreState::default();

    state.create_credential_group("生产密钥".to_owned(), CredentialKind::PrivateKey, None);
    let parent_id = state.storage.credential_groups[0].id;
    let outcome = state.create_credential_group(
        "API".to_owned(),
        CredentialKind::PrivateKey,
        Some(parent_id),
    );

    assert!(outcome.changed());
    assert_eq!(state.storage.credential_group_count(), 2);
    assert_eq!(
        state.storage.credential_groups[1].parent_id,
        Some(parent_id)
    );
}

#[test]
fn create_credential_group_rejects_missing_parent() {
    let mut state = CoreState::default();

    let outcome = state.create_credential_group(
        "API".to_owned(),
        CredentialKind::PrivateKey,
        Some(CredentialGroupId(uuid::Uuid::nil())),
    );

    assert!(outcome.error.is_some());
    assert_eq!(state.storage.credential_group_count(), 0);
}

#[test]
fn rename_credential_group_updates_existing_group_name() {
    let mut state = CoreState::default();

    state.create_credential_group("生产密钥".to_owned(), CredentialKind::PrivateKey, None);
    let group_id = state.storage.credential_groups[0].id;
    let outcome = state.rename_credential_group(group_id, "生产证书".to_owned());

    assert!(outcome.changed());
    assert_eq!(state.storage.credential_groups[0].name, "生产证书");
}

#[test]
fn remove_credential_group_reports_state_change_and_failure() {
    let mut state = CoreState::default();

    state.create_credential_group("生产密钥".to_owned(), CredentialKind::PrivateKey, None);
    let group_id = state.storage.credential_groups[0].id;

    let removed = state.remove_credential_group(group_id);

    assert!(removed.changed());
    assert_eq!(state.storage.credential_group_count(), 0);

    let missing = state.remove_credential_group(CredentialGroupId(uuid::Uuid::nil()));
    assert!(missing.error.is_some());
    assert_eq!(state.storage.credential_group_count(), 0);
}

#[test]
fn remove_credential_group_rejects_group_with_children() {
    let mut state = CoreState::default();

    state.create_credential_group("生产密钥".to_owned(), CredentialKind::PrivateKey, None);
    let parent_id = state.storage.credential_groups[0].id;
    state.create_credential_group(
        "API".to_owned(),
        CredentialKind::PrivateKey,
        Some(parent_id),
    );

    let outcome = state.remove_credential_group(parent_id);

    assert!(outcome.error.is_some());
    assert_eq!(state.storage.credential_group_count(), 2);
}

#[test]
fn create_private_key_credential_metadata_saves_record() {
    let mut state = CoreState::default();

    let outcome = state.create_credential_metadata(
        CredentialKind::PrivateKey,
        "deploy key".to_owned(),
        None,
        "secret://keys/deploy".to_owned(),
        Some(KeyAlgorithm::Ed25519),
    );

    assert!(outcome.changed());
    assert_eq!(state.storage.credential_count(), 1);
    assert_eq!(state.storage.credentials[0].name, "deploy key");
}

#[test]
fn create_private_key_credential_metadata_can_target_group() {
    let mut state = CoreState::default();
    state.create_credential_group("生产密钥".to_owned(), CredentialKind::PrivateKey, None);
    let group_id = state.storage.credential_groups[0].id;

    let outcome = state.create_credential_metadata(
        CredentialKind::PrivateKey,
        "deploy key".to_owned(),
        Some(group_id),
        "secret://keys/deploy".to_owned(),
        Some(KeyAlgorithm::Ed25519),
    );

    assert!(outcome.changed());
    assert_eq!(state.storage.credentials[0].group_id, Some(group_id));
}

#[test]
fn import_private_key_credential_saves_secret_payload_and_metadata() {
    let mut state = CoreState::default();
    state.create_credential_group("生产密钥".to_owned(), CredentialKind::PrivateKey, None);
    let group_id = state.storage.credential_groups[0].id;
    let path = std::env::temp_dir().join(format!(
        "smagicalssh-private-key-{}.key",
        uuid::Uuid::new_v4()
    ));
    let payload = private_key_fixture_payload();
    fs::write(&path, payload.as_bytes()).expect("private key fixture should write");

    let outcome = state.import_private_key_credential(
        "deploy key".to_owned(),
        Some(group_id),
        path.display().to_string(),
        Some(KeyAlgorithm::Ed25519),
    );

    assert!(outcome.changed());
    assert_eq!(state.storage.credential_count(), 1);
    assert_eq!(state.storage.secret_count(), 1);
    let credential = &state.storage.credentials[0];
    let secret = &state.storage.secrets[0];
    assert_eq!(credential.group_id, Some(group_id));
    assert_eq!(credential.secret, Some(secret.secret_ref.clone()));
    assert_eq!(credential.key_algorithm, Some(KeyAlgorithm::Ed25519));
    assert_eq!(secret.kind, SecretMaterialKind::PrivateKey);
    assert_eq!(
        secret.encrypted_payload.as_deref(),
        Some(payload.as_bytes())
    );
    let inspection = &state.storage.credential_inspections[0];
    assert_eq!(inspection.credential_id, credential.id);
    assert_eq!(inspection.kind, CredentialKind::PrivateKey);
    assert!(inspection.parse_error.is_none());
    assert_eq!(inspection.algorithm, Some(KeyAlgorithm::Ed25519));
    assert!(inspection.public_key.is_some());
    let _ = fs::remove_file(path);
}

#[test]
fn import_private_key_text_credential_saves_secret_payload_and_metadata() {
    let mut state = CoreState::default();
    state.create_credential_group("生产密钥".to_owned(), CredentialKind::PrivateKey, None);
    let group_id = state.storage.credential_groups[0].id;
    let payload = private_key_fixture_payload();

    let outcome = state.import_private_key_text_credential(
        "deploy key".to_owned(),
        Some(group_id),
        payload.clone(),
        Some(KeyAlgorithm::Ed25519),
    );

    assert!(outcome.changed());
    assert_eq!(state.storage.credential_count(), 1);
    assert_eq!(state.storage.secret_count(), 1);
    let credential = &state.storage.credentials[0];
    let secret = &state.storage.secrets[0];
    assert_eq!(credential.group_id, Some(group_id));
    assert_eq!(credential.secret, Some(secret.secret_ref.clone()));
    assert_eq!(credential.key_algorithm, Some(KeyAlgorithm::Ed25519));
    assert_eq!(secret.kind, SecretMaterialKind::PrivateKey);
    assert_eq!(
        secret.encrypted_payload.as_deref(),
        Some(payload.as_bytes())
    );
    assert!(
        state.storage.credential_inspections[0]
            .parse_error
            .is_none()
    );
}

#[test]
fn import_private_key_text_credential_detects_algorithm_from_payload() {
    let mut state = CoreState::default();
    let mut rng = OsRng;
    let private_key = PrivateKey::random(&mut rng, Algorithm::Ed25519)
        .expect("private key fixture should generate");
    let payload = private_key
        .to_openssh(LineEnding::LF)
        .expect("private key fixture should encode");

    let outcome = state.import_private_key_text_credential(
        "detected key".to_owned(),
        None,
        payload.to_string(),
        Some(KeyAlgorithm::Rsa),
    );

    assert!(outcome.changed());
    assert_eq!(
        state.storage.credentials[0].key_algorithm,
        Some(KeyAlgorithm::Ed25519)
    );
}

#[test]
fn import_certificate_text_credential_detects_algorithm_from_openssh_token() {
    let mut state = CoreState::default();
    let payload = certificate_fixture_payload();

    let outcome = state.import_certificate_text_credential(
        "detected cert".to_owned(),
        None,
        payload,
        Some(KeyAlgorithm::Rsa),
    );

    assert!(outcome.changed());
    assert_eq!(
        state.storage.credentials[0].key_algorithm,
        Some(KeyAlgorithm::Ed25519)
    );
}

#[test]
fn generate_private_key_credential_saves_openssh_key_payload() {
    let mut state = CoreState::default();
    state.create_credential_group("生产密钥".to_owned(), CredentialKind::PrivateKey, None);
    let group_id = state.storage.credential_groups[0].id;

    let outcome = state.generate_private_key_credential(
        "deploy key".to_owned(),
        Some(group_id),
        Some(KeyAlgorithm::Ed25519),
    );

    assert!(outcome.changed());
    assert_eq!(state.storage.credential_count(), 1);
    assert_eq!(state.storage.secret_count(), 1);
    let credential = &state.storage.credentials[0];
    let secret = &state.storage.secrets[0];
    assert_eq!(credential.group_id, Some(group_id));
    assert_eq!(credential.secret, Some(secret.secret_ref.clone()));
    assert_eq!(credential.key_algorithm, Some(KeyAlgorithm::Ed25519));
    assert!(
        credential
            .fingerprint
            .as_ref()
            .is_some_and(|fingerprint| fingerprint.starts_with("SHA256:"))
    );
    assert_eq!(secret.kind, SecretMaterialKind::PrivateKey);
    let payload = std::str::from_utf8(
        secret
            .encrypted_payload
            .as_deref()
            .expect("generated private key payload should exist"),
    )
    .expect("generated private key should be utf-8");
    assert!(payload.starts_with("-----BEGIN OPENSSH PRIVATE KEY-----"));
    russh::keys::decode_secret_key(payload, None).expect("generated private key should decode");
    let inspection = &state.storage.credential_inspections[0];
    assert_eq!(inspection.credential_id, credential.id);
    assert_eq!(inspection.kind, CredentialKind::PrivateKey);
    assert!(inspection.parse_error.is_none());
    assert_eq!(inspection.algorithm, Some(KeyAlgorithm::Ed25519));
    assert_eq!(inspection.fingerprint, credential.fingerprint);
    assert!(inspection.public_key.is_some());
}

#[test]
fn generate_private_key_credential_supports_all_selectable_algorithms() {
    for algorithm in [
        KeyAlgorithm::Ed25519,
        KeyAlgorithm::Rsa,
        KeyAlgorithm::Ecdsa,
    ] {
        let mut state = CoreState::default();
        let name = format!("generated-{algorithm:?}");

        let outcome = state.generate_private_key_credential(name, None, Some(algorithm.clone()));

        assert!(outcome.changed(), "{algorithm:?} should generate a key");
        assert_eq!(state.storage.credential_count(), 1);
        assert_eq!(state.storage.secret_count(), 1);

        let credential = &state.storage.credentials[0];
        let secret = &state.storage.secrets[0];
        assert_eq!(credential.key_algorithm, Some(algorithm.clone()));
        assert_eq!(secret.kind, SecretMaterialKind::PrivateKey);

        let payload = std::str::from_utf8(
            secret
                .encrypted_payload
                .as_deref()
                .expect("generated private key payload should exist"),
        )
        .expect("generated private key should be utf-8");
        let decoded =
            russh::keys::decode_secret_key(payload, None).expect("generated key should decode");
        assert_eq!(
            KeyAlgorithm::from_ssh_algorithm(decoded.algorithm().as_str()),
            algorithm
        );
    }
}

#[test]
fn generate_certificate_credential_signs_subject_key_and_saves_certificate_payload() {
    let mut state = CoreState::default();
    state.create_credential_group("生产证书".to_owned(), CredentialKind::Certificate, None);
    let certificate_group_id = state.storage.credential_groups[0].id;
    state.generate_private_key_credential("ca key".to_owned(), None, Some(KeyAlgorithm::Ed25519));
    state.generate_private_key_credential(
        "subject key".to_owned(),
        None,
        Some(KeyAlgorithm::Ed25519),
    );
    let ca_ref = credential_secret_ref(&state, "ca key");
    let subject_ref = credential_secret_ref(&state, "subject key");

    let outcome = state.generate_certificate_credential(
        "deploy cert".to_owned(),
        Some(certificate_group_id),
        ca_ref,
        subject_ref,
        "User".to_owned(),
        "deploy, root deploy".to_owned(),
        "30".to_owned(),
        "deploy-cert-01".to_owned(),
        "42".to_owned(),
    );

    assert!(outcome.changed());
    let credential = state
        .storage
        .credentials
        .iter()
        .find(|credential| credential.name == "deploy cert")
        .expect("certificate credential should exist");
    assert_eq!(credential.kind, CredentialKind::Certificate);
    assert_eq!(credential.group_id, Some(certificate_group_id));
    assert_eq!(credential.key_algorithm, Some(KeyAlgorithm::Ed25519));
    assert!(
        credential
            .fingerprint
            .as_ref()
            .is_some_and(|fingerprint| fingerprint.starts_with("SHA256:"))
    );

    let secret = state
        .storage
        .secrets
        .iter()
        .find(|secret| credential.secret.as_ref() == Some(&secret.secret_ref))
        .expect("certificate secret should exist");
    assert_eq!(secret.kind, SecretMaterialKind::Certificate);
    let payload = std::str::from_utf8(
        secret
            .encrypted_payload
            .as_deref()
            .expect("certificate payload should exist"),
    )
    .expect("certificate payload should be utf-8");
    let certificate = Certificate::from_openssh(payload)
        .expect("generated certificate should decode as OpenSSH certificate");
    assert_eq!(certificate.cert_type(), certificate::CertType::User);
    assert_eq!(certificate.serial(), 42);
    assert_eq!(certificate.key_id(), "deploy-cert-01");
    assert_eq!(
        certificate.valid_principals(),
        &["deploy".to_owned(), "root".to_owned(),]
    );
    let inspection = state
        .storage
        .credential_inspections
        .iter()
        .find(|inspection| inspection.credential_id == credential.id)
        .expect("certificate inspection should exist");
    let certificate_inspection = inspection
        .certificate
        .as_ref()
        .expect("certificate inspection details should exist");
    assert!(inspection.parse_error.is_none());
    assert_eq!(certificate_inspection.serial, Some(42));
    assert_eq!(
        certificate_inspection.principals,
        vec!["deploy".to_owned(), "root".to_owned()]
    );
}

#[test]
fn generate_certificate_credential_rejects_missing_principal() {
    let mut state = CoreState::default();
    state.generate_private_key_credential("ca key".to_owned(), None, Some(KeyAlgorithm::Ed25519));
    state.generate_private_key_credential(
        "subject key".to_owned(),
        None,
        Some(KeyAlgorithm::Ed25519),
    );
    let ca_ref = credential_secret_ref(&state, "ca key");
    let subject_ref = credential_secret_ref(&state, "subject key");

    let outcome = state.generate_certificate_credential(
        "deploy cert".to_owned(),
        None,
        ca_ref,
        subject_ref,
        "User".to_owned(),
        " ".to_owned(),
        "30".to_owned(),
        "deploy-cert-01".to_owned(),
        "42".to_owned(),
    );

    assert!(!outcome.state_changed);
    assert_eq!(outcome.error.as_deref(), Some("Principal 不能为空"));
    assert!(
        state
            .storage
            .credentials
            .iter()
            .all(|credential| credential.kind != CredentialKind::Certificate)
    );
}

#[test]
fn generate_certificate_credential_rejects_mismatched_group_kind() {
    let mut state = CoreState::default();
    state.create_credential_group("生产密钥".to_owned(), CredentialKind::PrivateKey, None);
    let private_key_group_id = state.storage.credential_groups[0].id;
    state.generate_private_key_credential("ca key".to_owned(), None, Some(KeyAlgorithm::Ed25519));
    state.generate_private_key_credential(
        "subject key".to_owned(),
        None,
        Some(KeyAlgorithm::Ed25519),
    );
    let ca_ref = credential_secret_ref(&state, "ca key");
    let subject_ref = credential_secret_ref(&state, "subject key");

    let outcome = state.generate_certificate_credential(
        "deploy cert".to_owned(),
        Some(private_key_group_id),
        ca_ref,
        subject_ref,
        "User".to_owned(),
        "deploy".to_owned(),
        "30".to_owned(),
        "deploy-cert-01".to_owned(),
        "42".to_owned(),
    );

    assert!(!outcome.state_changed);
    assert_eq!(outcome.error.as_deref(), Some("密钥分组类型不匹配"));
    assert!(!state.storage.credentials.iter().any(|credential| {
        credential.name == "deploy cert" && credential.kind == CredentialKind::Certificate
    }));
}

#[test]
fn save_password_credential_saves_secret_payload_and_metadata() {
    let mut state = CoreState::default();
    state.create_credential_group("生产密码".to_owned(), CredentialKind::Password, None);
    let group_id = state.storage.credential_groups[0].id;

    let outcome = state.save_password_credential(
        "deploy password".to_owned(),
        Some(group_id),
        "s3cr3t".to_owned(),
    );

    assert!(outcome.changed());
    assert_eq!(state.storage.credential_count(), 1);
    assert_eq!(state.storage.secret_count(), 1);
    let credential = &state.storage.credentials[0];
    let secret = &state.storage.secrets[0];
    assert_eq!(credential.kind, CredentialKind::Password);
    assert_eq!(credential.group_id, Some(group_id));
    assert_eq!(credential.secret, Some(secret.secret_ref.clone()));
    assert_eq!(secret.kind, SecretMaterialKind::Password);
    assert_eq!(
        secret.encrypted_payload.as_deref(),
        Some(b"s3cr3t".as_slice())
    );
    let inspection = &state.storage.credential_inspections[0];
    assert_eq!(inspection.credential_id, credential.id);
    assert_eq!(inspection.kind, CredentialKind::Password);
    assert!(inspection.parse_error.is_none());
    assert_eq!(inspection.password_length, Some(6));
    assert!(inspection.public_key.is_none());
}

#[test]
fn import_certificate_credential_saves_secret_payload_and_metadata() {
    let mut state = CoreState::default();
    state.create_credential_group("生产证书".to_owned(), CredentialKind::Certificate, None);
    let group_id = state.storage.credential_groups[0].id;
    let path = std::env::temp_dir().join(format!(
        "smagicalssh-certificate-{}.pub",
        uuid::Uuid::new_v4()
    ));
    let payload = certificate_fixture_payload();
    fs::write(&path, payload.as_bytes()).expect("certificate fixture should write");

    let outcome = state.import_certificate_credential(
        "deploy cert".to_owned(),
        Some(group_id),
        path.display().to_string(),
        Some(KeyAlgorithm::Ed25519),
    );

    assert!(outcome.changed());
    assert_eq!(state.storage.credential_count(), 1);
    assert_eq!(state.storage.secret_count(), 1);
    let credential = &state.storage.credentials[0];
    let secret = &state.storage.secrets[0];
    assert_eq!(credential.kind, CredentialKind::Certificate);
    assert_eq!(credential.group_id, Some(group_id));
    assert_eq!(credential.secret, Some(secret.secret_ref.clone()));
    assert_eq!(credential.key_algorithm, Some(KeyAlgorithm::Ed25519));
    assert_eq!(secret.kind, SecretMaterialKind::Certificate);
    assert_eq!(
        secret.encrypted_payload.as_deref(),
        Some(payload.as_bytes())
    );
    let inspection = &state.storage.credential_inspections[0];
    assert_eq!(inspection.credential_id, credential.id);
    assert_eq!(inspection.kind, CredentialKind::Certificate);
    assert!(inspection.parse_error.is_none());
    assert_eq!(inspection.algorithm, Some(KeyAlgorithm::Ed25519));
    assert_eq!(
        inspection
            .certificate
            .as_ref()
            .and_then(|certificate| certificate.serial),
        Some(42)
    );
    let _ = fs::remove_file(path);
}

#[test]
fn import_certificate_text_credential_saves_secret_payload_and_metadata() {
    let mut state = CoreState::default();
    state.create_credential_group("生产证书".to_owned(), CredentialKind::Certificate, None);
    let group_id = state.storage.credential_groups[0].id;
    let payload = certificate_fixture_payload();

    let outcome = state.import_certificate_text_credential(
        "deploy cert".to_owned(),
        Some(group_id),
        payload.clone(),
        Some(KeyAlgorithm::Ed25519),
    );

    assert!(outcome.changed());
    assert_eq!(state.storage.credential_count(), 1);
    assert_eq!(state.storage.secret_count(), 1);
    let credential = &state.storage.credentials[0];
    let secret = &state.storage.secrets[0];
    assert_eq!(credential.kind, CredentialKind::Certificate);
    assert_eq!(credential.group_id, Some(group_id));
    assert_eq!(credential.secret, Some(secret.secret_ref.clone()));
    assert_eq!(credential.key_algorithm, Some(KeyAlgorithm::Ed25519));
    assert_eq!(secret.kind, SecretMaterialKind::Certificate);
    assert_eq!(
        secret.encrypted_payload.as_deref(),
        Some(payload.as_bytes())
    );
    assert!(
        state.storage.credential_inspections[0]
            .parse_error
            .is_none()
    );
}

#[test]
fn import_credential_rejects_unparseable_private_key_and_certificate() {
    let mut state = CoreState::default();

    let private_key = state.import_private_key_text_credential(
        "bad key".to_owned(),
        None,
        "not a private key".to_owned(),
        Some(KeyAlgorithm::Ed25519),
    );
    let certificate = state.import_certificate_text_credential(
        "bad cert".to_owned(),
        None,
        "not a certificate".to_owned(),
        Some(KeyAlgorithm::Ed25519),
    );

    assert!(private_key.error.is_some());
    assert!(certificate.error.is_some());
    assert_eq!(state.storage.credential_count(), 0);
    assert_eq!(state.storage.secret_count(), 0);
    assert_eq!(state.storage.credential_inspections.len(), 0);
}

#[test]
fn create_credential_metadata_rejects_mismatched_group_kind() {
    let mut state = CoreState::default();
    state.create_credential_group("证书".to_owned(), CredentialKind::Certificate, None);
    let group_id = state.storage.credential_groups[0].id;

    let outcome = state.create_credential_metadata(
        CredentialKind::PrivateKey,
        "deploy key".to_owned(),
        Some(group_id),
        "secret://keys/deploy".to_owned(),
        Some(KeyAlgorithm::Ed25519),
    );

    assert!(outcome.error.is_some());
    assert_eq!(state.storage.credential_count(), 0);
}

#[test]
fn create_certificate_credential_metadata_saves_record() {
    let mut state = CoreState::default();

    let outcome = state.create_credential_metadata(
        CredentialKind::Certificate,
        "deploy cert".to_owned(),
        None,
        "secret://certs/deploy".to_owned(),
        Some(KeyAlgorithm::Ed25519),
    );

    assert!(outcome.changed());
    assert_eq!(state.storage.credential_count(), 1);
}

#[test]
fn create_credential_metadata_rejects_empty_required_fields() {
    let mut state = CoreState::default();

    assert!(
        state
            .create_credential_metadata(
                CredentialKind::PrivateKey,
                "".to_owned(),
                None,
                "secret://keys/deploy".to_owned(),
                Some(KeyAlgorithm::Ed25519),
            )
            .error
            .is_some()
    );
    assert!(
        state
            .create_credential_metadata(
                CredentialKind::Certificate,
                "deploy cert".to_owned(),
                None,
                "".to_owned(),
                Some(KeyAlgorithm::Ed25519),
            )
            .error
            .is_some()
    );
    assert_eq!(state.storage.credential_count(), 0);
}

#[test]
fn update_credential_metadata_renames_without_touching_secret_payload() {
    let mut state = CoreState::default();
    let secret_ref = SecretRef("secret://keys/deploy".to_owned());
    state.storage.upsert_secret(SecretRecord::local_plaintext(
        secret_ref.clone(),
        SecretMaterialKind::PrivateKey,
        b"private-key".to_vec(),
    ));
    state.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "deploy".to_owned(),
        kind: CredentialKind::PrivateKey,
        group_id: None,
        username: Some("deploy".to_owned()),
        secret: Some(secret_ref.clone()),
        key_algorithm: Some(KeyAlgorithm::Ed25519),
        fingerprint: Some("SHA256:key".to_owned()),
    });

    let outcome = state.update_credential_metadata(
        "deploy",
        "deploy-prod".to_owned(),
        None,
        Some(KeyAlgorithm::Rsa),
    );

    assert!(outcome.changed());
    assert_eq!(state.storage.credential_count(), 1);
    assert_eq!(state.storage.secret_count(), 1);
    let credential = &state.storage.credentials[0];
    assert_eq!(credential.name, "deploy-prod");
    assert_eq!(credential.secret, Some(secret_ref.clone()));
    assert_eq!(credential.username, Some("deploy".to_owned()));
    assert_eq!(credential.fingerprint, Some("SHA256:key".to_owned()));
    assert_eq!(credential.key_algorithm, Some(KeyAlgorithm::Rsa));
    assert_eq!(state.storage.secrets[0].secret_ref, secret_ref);
}

#[test]
fn update_credential_secret_refreshes_payload_metadata_and_inspection() {
    let mut state = CoreState::default();
    let secret_ref = SecretRef("secret://keys/deploy".to_owned());
    state.storage.upsert_secret(SecretRecord::local_plaintext(
        secret_ref.clone(),
        SecretMaterialKind::PrivateKey,
        b"old-key\n".to_vec(),
    ));
    state.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "deploy".to_owned(),
        kind: CredentialKind::PrivateKey,
        group_id: None,
        username: Some("deploy".to_owned()),
        secret: Some(secret_ref.clone()),
        key_algorithm: Some(KeyAlgorithm::Ed25519),
        fingerprint: Some("SHA256:old".to_owned()),
    });
    let payload = private_key_fixture_payload();

    let outcome = state.update_credential_secret("deploy", payload.clone());

    assert!(outcome.changed());
    assert_eq!(state.storage.credential_count(), 1);
    assert_eq!(state.storage.secret_count(), 1);
    assert_eq!(
        state.storage.secrets[0].encrypted_payload.as_deref(),
        Some(payload.as_bytes())
    );
    let credential = &state.storage.credentials[0];
    assert_eq!(credential.name, "deploy");
    assert_eq!(credential.secret, Some(secret_ref.clone()));
    assert_eq!(credential.username, Some("deploy".to_owned()));
    assert_eq!(credential.key_algorithm, Some(KeyAlgorithm::Ed25519));
    assert!(
        credential
            .fingerprint
            .as_ref()
            .is_some_and(|fingerprint| fingerprint.starts_with("SHA256:"))
    );
    assert_ne!(credential.fingerprint, Some("SHA256:old".to_owned()));
    let inspection = &state.storage.credential_inspections[0];
    assert_eq!(inspection.credential_id, credential.id);
    assert!(inspection.parse_error.is_none());
    assert_eq!(inspection.fingerprint, credential.fingerprint);
}

#[test]
fn update_credential_secret_replaces_password_without_trimming() {
    let mut state = CoreState::default();
    let secret_ref = SecretRef("secret://passwords/admin".to_owned());
    state.storage.upsert_secret(SecretRecord::local_plaintext(
        secret_ref.clone(),
        SecretMaterialKind::Password,
        b"old".to_vec(),
    ));
    state.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "admin".to_owned(),
        kind: CredentialKind::Password,
        group_id: None,
        username: None,
        secret: Some(secret_ref),
        key_algorithm: None,
        fingerprint: None,
    });

    let outcome = state.update_credential_secret("admin", " new password ".to_owned());

    assert!(outcome.changed());
    assert_eq!(
        state.storage.secrets[0].encrypted_payload.as_deref(),
        Some(b" new password ".as_slice())
    );
    assert_eq!(state.storage.credential_inspections.len(), 1);
    assert_eq!(
        state.storage.credential_inspections[0].password_length,
        Some(14)
    );
}

#[test]
fn update_credential_secret_rejects_missing_or_unsupported_payload() {
    let mut state = CoreState::default();
    state.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "agent".to_owned(),
        kind: CredentialKind::Agent,
        group_id: None,
        username: None,
        secret: Some(SecretRef("agent://auto".to_owned())),
        key_algorithm: None,
        fingerprint: None,
    });

    let missing = state.update_credential_secret("missing", "value".to_owned());
    assert!(!missing.state_changed);
    assert_eq!(missing.error.as_deref(), Some("找不到凭据：missing"));

    let unsupported = state.update_credential_secret("agent", "agent://pageant".to_owned());
    assert!(!unsupported.state_changed);
    assert_eq!(
        unsupported.error.as_deref(),
        Some("该凭据没有可替换的本地内容")
    );
}

#[test]
fn update_credential_secret_rejects_unparseable_private_key_without_overwriting() {
    let mut state = CoreState::default();
    let secret_ref = SecretRef("secret://keys/deploy".to_owned());
    let old_payload = private_key_fixture_payload();
    state.storage.upsert_secret(SecretRecord::local_plaintext(
        secret_ref.clone(),
        SecretMaterialKind::PrivateKey,
        old_payload.as_bytes().to_vec(),
    ));
    state.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "deploy".to_owned(),
        kind: CredentialKind::PrivateKey,
        group_id: None,
        username: None,
        secret: Some(secret_ref),
        key_algorithm: Some(KeyAlgorithm::Ed25519),
        fingerprint: Some("SHA256:old".to_owned()),
    });

    let outcome = state.update_credential_secret("deploy", "not a private key".to_owned());

    assert!(outcome.error.is_some());
    assert_eq!(state.storage.credential_inspections.len(), 0);
    assert_eq!(
        state.storage.secrets[0].encrypted_payload.as_deref(),
        Some(old_payload.as_bytes())
    );
    assert_eq!(
        state.storage.credentials[0].fingerprint,
        Some("SHA256:old".to_owned())
    );
}

#[test]
fn update_credential_secret_can_restore_missing_local_payload() {
    let mut state = CoreState::default();
    let secret_ref = SecretRef("secret://certs/deploy".to_owned());
    state.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "deploy-cert".to_owned(),
        kind: CredentialKind::Certificate,
        group_id: None,
        username: None,
        secret: Some(secret_ref.clone()),
        key_algorithm: Some(KeyAlgorithm::Ed25519),
        fingerprint: None,
    });
    let payload = certificate_fixture_payload();

    let outcome = state.update_credential_secret("deploy-cert", payload.clone());

    assert!(outcome.changed());
    assert_eq!(state.storage.secret_count(), 1);
    assert_eq!(state.storage.secrets[0].secret_ref, secret_ref.clone());
    assert_eq!(
        state.storage.secrets[0].kind,
        SecretMaterialKind::Certificate
    );
    assert_eq!(
        state.storage.secrets[0].encrypted_payload.as_deref(),
        Some(payload.as_bytes())
    );
    assert!(
        state.storage.credential_inspections[0]
            .parse_error
            .is_none()
    );
}

#[test]
fn update_credential_metadata_moves_to_matching_group() {
    let mut state = CoreState::default();
    state.create_credential_group("生产密钥".to_owned(), CredentialKind::PrivateKey, None);
    let group_id = state.storage.credential_groups[0].id;
    state.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "deploy".to_owned(),
        kind: CredentialKind::PrivateKey,
        group_id: None,
        username: None,
        secret: Some(SecretRef("secret://keys/deploy".to_owned())),
        key_algorithm: Some(KeyAlgorithm::Ed25519),
        fingerprint: None,
    });

    let outcome = state.update_credential_metadata(
        "deploy",
        "deploy".to_owned(),
        Some(group_id),
        Some(KeyAlgorithm::Ecdsa),
    );

    assert!(outcome.changed());
    assert_eq!(state.storage.credentials[0].group_id, Some(group_id));
    assert_eq!(
        state.storage.credentials[0].key_algorithm,
        Some(KeyAlgorithm::Ecdsa)
    );
}

#[test]
fn update_credential_metadata_rejects_duplicate_name() {
    let mut state = CoreState::default();
    state.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "deploy".to_owned(),
        kind: CredentialKind::Password,
        group_id: None,
        username: None,
        secret: Some(SecretRef("secret://passwords/deploy".to_owned())),
        key_algorithm: None,
        fingerprint: None,
    });
    state.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "admin".to_owned(),
        kind: CredentialKind::Password,
        group_id: None,
        username: None,
        secret: Some(SecretRef("secret://passwords/admin".to_owned())),
        key_algorithm: None,
        fingerprint: None,
    });

    let outcome = state.update_credential_metadata("deploy", "admin".to_owned(), None, None);

    assert!(outcome.error.is_some());
    assert_eq!(state.storage.credentials[0].name, "deploy");
    assert_eq!(state.storage.credential_count(), 2);
}

#[test]
fn update_credential_metadata_rejects_mismatched_group_kind() {
    let mut state = CoreState::default();
    state.create_credential_group("证书".to_owned(), CredentialKind::Certificate, None);
    let group_id = state.storage.credential_groups[0].id;
    state.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "deploy".to_owned(),
        kind: CredentialKind::PrivateKey,
        group_id: None,
        username: None,
        secret: Some(SecretRef("secret://keys/deploy".to_owned())),
        key_algorithm: Some(KeyAlgorithm::Ed25519),
        fingerprint: None,
    });

    let outcome = state.update_credential_metadata(
        "deploy",
        "deploy".to_owned(),
        Some(group_id),
        Some(KeyAlgorithm::Ed25519),
    );

    assert!(outcome.error.is_some());
    assert!(state.storage.credentials[0].group_id.is_none());
}

#[test]
fn move_credential_updates_group_when_target_kind_matches() {
    let mut state = CoreState::default();
    state.create_credential_group("生产密钥".to_owned(), CredentialKind::PrivateKey, None);
    let group_id = state.storage.credential_groups[0].id;
    state.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "deploy".to_owned(),
        kind: CredentialKind::PrivateKey,
        group_id: None,
        username: None,
        secret: Some(SecretRef("secret://keys/deploy".to_owned())),
        key_algorithm: Some(KeyAlgorithm::Ed25519),
        fingerprint: None,
    });

    let outcome = state.move_credential("deploy", Some(group_id));

    assert!(outcome.changed());
    assert_eq!(state.storage.credentials[0].group_id, Some(group_id));
}

#[test]
fn move_credential_group_rejects_descendant_target() {
    let mut state = CoreState::default();
    state.create_credential_group("生产密钥".to_owned(), CredentialKind::PrivateKey, None);
    let parent_id = state.storage.credential_groups[0].id;
    state.create_credential_group(
        "API".to_owned(),
        CredentialKind::PrivateKey,
        Some(parent_id),
    );
    let child_id = state.storage.credential_groups[1].id;

    let outcome = state.move_credential_group(parent_id, Some(child_id));

    assert!(outcome.error.is_some());
    assert!(state.storage.credential_groups[0].parent_id.is_none());
}

#[test]
fn remove_credential_reports_state_change_and_failure() {
    let mut state = CoreState::default();
    state.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "deploy".to_owned(),
        kind: CredentialKind::Password,
        group_id: None,
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
fn remove_credential_deletes_unreferenced_secret_payload() {
    let mut state = CoreState::default();
    let secret_ref = SecretRef("secret://keys/deploy".to_owned());
    state.storage.upsert_secret(SecretRecord::local_plaintext(
        secret_ref.clone(),
        SecretMaterialKind::PrivateKey,
        b"private-key".to_vec(),
    ));
    state.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "deploy".to_owned(),
        kind: CredentialKind::PrivateKey,
        group_id: None,
        username: None,
        secret: Some(secret_ref),
        key_algorithm: Some(KeyAlgorithm::Ed25519),
        fingerprint: None,
    });

    let outcome = state.remove_credential("deploy");

    assert!(outcome.changed());
    assert_eq!(state.storage.credential_count(), 0);
    assert_eq!(state.storage.secret_count(), 0);
}

#[test]
fn export_credential_secret_writes_local_payload() {
    let mut state = CoreState::default();
    let secret_ref = SecretRef("secret://keys/deploy".to_owned());
    let payload = b"private-key";
    let target_path = std::env::temp_dir().join(format!(
        "smagicalssh-exported-key-{}.key",
        uuid::Uuid::new_v4()
    ));
    state.storage.upsert_secret(SecretRecord::local_plaintext(
        secret_ref.clone(),
        SecretMaterialKind::PrivateKey,
        payload.to_vec(),
    ));
    state.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "deploy".to_owned(),
        kind: CredentialKind::PrivateKey,
        group_id: None,
        username: None,
        secret: Some(secret_ref),
        key_algorithm: Some(KeyAlgorithm::Ed25519),
        fingerprint: None,
    });

    let outcome = state.export_credential_secret("deploy", &target_path.display().to_string());

    assert!(outcome.error.is_none());
    assert_eq!(
        fs::read(&target_path).expect("exported payload should read"),
        payload
    );

    let _ = fs::remove_file(target_path);
}

#[test]
fn duplicate_credential_copies_group_and_generates_unique_name() {
    let mut state = CoreState::default();
    state.create_credential_group("生产密钥".to_owned(), CredentialKind::PrivateKey, None);
    let group_id = state.storage.credential_groups[0].id;
    state.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "deploy".to_owned(),
        kind: CredentialKind::PrivateKey,
        group_id: Some(group_id),
        username: Some("deploy".to_owned()),
        secret: Some(SecretRef("key:deploy".to_owned())),
        key_algorithm: Some(KeyAlgorithm::Ed25519),
        fingerprint: Some("SHA256:key".to_owned()),
    });

    let outcome = state.duplicate_credential("deploy");

    assert!(outcome.changed());
    assert_eq!(state.storage.credential_count(), 2);
    let duplicate = state
        .storage
        .credentials
        .iter()
        .find(|credential| credential.name == "deploy 复制")
        .expect("duplicate credential should exist");
    assert_eq!(duplicate.group_id, Some(group_id));
    assert_eq!(duplicate.secret, Some(SecretRef("key:deploy".to_owned())));
}

#[test]
fn duplicate_credential_copies_local_secret_payload() {
    let mut state = CoreState::default();
    let secret_ref = SecretRef("secret://keys/deploy".to_owned());
    let payload = private_key_fixture_payload().into_bytes();
    state.storage.upsert_secret(SecretRecord::local_plaintext(
        secret_ref.clone(),
        SecretMaterialKind::PrivateKey,
        payload.clone(),
    ));
    state.storage.upsert_credential(CredentialMetadata {
        id: crate::model::CredentialId(uuid::Uuid::new_v4()),
        name: "deploy".to_owned(),
        kind: CredentialKind::PrivateKey,
        group_id: None,
        username: None,
        secret: Some(secret_ref.clone()),
        key_algorithm: Some(KeyAlgorithm::Ed25519),
        fingerprint: None,
    });

    let outcome = state.duplicate_credential("deploy");

    assert!(outcome.changed());
    assert_eq!(state.storage.credential_count(), 2);
    assert_eq!(state.storage.secret_count(), 2);
    let duplicate = state
        .storage
        .credentials
        .iter()
        .find(|credential| credential.name == "deploy 复制")
        .expect("duplicate credential should exist");
    let duplicate_ref = duplicate
        .secret
        .as_ref()
        .expect("duplicate should use its own secret ref");
    assert_ne!(duplicate_ref, &secret_ref);
    let copied_secret = state
        .storage
        .secrets
        .iter()
        .find(|secret| &secret.secret_ref == duplicate_ref)
        .expect("duplicate secret should exist");
    assert_eq!(copied_secret.kind, SecretMaterialKind::PrivateKey);
    assert_eq!(copied_secret.encrypted_payload, Some(payload));
    let original = state
        .storage
        .credentials
        .iter()
        .find(|credential| credential.name == "deploy")
        .expect("original credential should exist");
    assert_ne!(duplicate.id, original.id);
    let inspection = state
        .storage
        .credential_inspections
        .iter()
        .find(|inspection| inspection.credential_id == duplicate.id)
        .expect("duplicate inspection should exist");
    assert!(inspection.parse_error.is_none());
    assert_eq!(inspection.kind, CredentialKind::PrivateKey);
}

#[test]
fn known_host_can_be_trusted_and_removed() {
    let mut state = CoreState::default();
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

fn sample_network_host(network: HostNetworkSelection) -> Host {
    Host {
        id: crate::model::HostId(uuid::Uuid::new_v4()),
        name: "生产主机".to_owned(),
        group_id: None,
        icon_key: "server".to_owned(),
        tags: Vec::new(),
        address: "prod.example.com".to_owned(),
        port: 22,
        auth: AuthProfile::Password {
            username: "ops".to_owned(),
            secret: SecretRef("password:ops".to_owned()),
        },
        network,
        proxies: Vec::new(),
        jumps: Vec::new(),
        theme_override: None,
        background_override: None,
    }
}

#[test]
fn network_proxy_asset_can_be_created_updated_and_removed() {
    let mut state = CoreState::default();

    let created = state.apply(Message::SaveProxyAsset {
        proxy_id: None,
        name: " 办公网关 ".to_owned(),
        proxy_kind: "Socks5".to_owned(),
        host: " 127.0.0.1 ".to_owned(),
        port: "1080".to_owned(),
        tags: "office, shared".to_owned(),
        auth_kind: "UserPassword".to_owned(),
        auth_username: "proxy-user".to_owned(),
        auth_password_ref: "proxy-password-1".to_owned(),
        remote_dns: true,
    });

    assert!(created.changed());
    assert_eq!(state.storage.proxy_asset_count(), 1);
    let proxy_id = state.storage.proxy_assets[0].id;
    assert_eq!(state.storage.proxy_assets[0].name, "办公网关");
    assert!(matches!(
        &state.storage.proxy_assets[0].profile,
        crate::model::ProxyProfile::Socks5 {
            auth: crate::model::ProxyAuth::UserPassword { username, password },
            remote_dns: true,
            ..
        } if username == "proxy-user"
            && password.as_ref().is_some_and(|secret| secret.0.starts_with("secret://network-proxies/"))
    ));
    assert_eq!(state.storage.secret_count(), 1);

    let updated = state.apply(Message::SaveProxyAsset {
        proxy_id: Some(proxy_id),
        name: "办公 HTTP".to_owned(),
        proxy_kind: "Http".to_owned(),
        host: "proxy.internal".to_owned(),
        port: "8080".to_owned(),
        tags: "office".to_owned(),
        auth_kind: "None".to_owned(),
        auth_username: String::new(),
        auth_password_ref: String::new(),
        remote_dns: false,
    });

    assert!(updated.changed());
    assert_eq!(state.storage.proxy_asset_count(), 1);
    assert_eq!(state.storage.proxy_assets[0].name, "办公 HTTP");

    let removed = state.apply(Message::RemoveProxyAsset { proxy_id });
    assert!(removed.changed());
    assert_eq!(state.storage.proxy_asset_count(), 0);
}

#[test]
fn referenced_network_proxy_asset_reports_used_hosts_on_delete() {
    let mut state = CoreState::default();
    state.apply(Message::SaveProxyAsset {
        proxy_id: None,
        name: "办公网关".to_owned(),
        proxy_kind: "Socks5".to_owned(),
        host: "127.0.0.1".to_owned(),
        port: "1080".to_owned(),
        tags: String::new(),
        auth_kind: "None".to_owned(),
        auth_username: String::new(),
        auth_password_ref: String::new(),
        remote_dns: false,
    });
    let proxy_id = state.storage.proxy_assets[0].id;
    state
        .storage
        .upsert_host(sample_network_host(HostNetworkSelection {
            proxy_ids: vec![proxy_id],
            jump_chain_ids: Vec::new(),
            forward_ids: Vec::new(),
        }));

    let outcome = state.apply(Message::RemoveProxyAsset { proxy_id });

    assert!(outcome.error.is_some());
    assert!(
        outcome
            .error
            .as_deref()
            .is_some_and(|error| error.contains("生产主机"))
    );
    assert_eq!(state.storage.proxy_asset_count(), 1);
}

#[test]
fn network_jump_chain_asset_validates_hosts_and_blocks_referenced_delete() {
    let mut state = CoreState::default();
    let host = sample_network_host(HostNetworkSelection::default());
    let host_id = host.id;
    state.storage.upsert_host(host);

    let created = state.apply(Message::SaveJumpChainAsset {
        chain_id: None,
        name: "生产跳板".to_owned(),
        steps: vec![crate::model::JumpProfile {
            host_id,
            username_override: None,
            port_override: None,
            alias: None,
        }],
    });

    assert!(created.changed());
    assert_eq!(state.storage.jump_chain_asset_count(), 1);
    let chain_id = state.storage.jump_chain_assets[0].id;
    state.storage.hosts[0].network.jump_chain_ids.push(chain_id);

    let outcome = state.apply(Message::RemoveJumpChainAsset { chain_id });

    assert!(outcome.error.is_some());
    assert_eq!(state.storage.jump_chain_asset_count(), 1);
}

#[test]
fn network_forward_asset_can_be_created_updated_and_removed() {
    let mut state = CoreState::default();

    let created = state.apply(Message::SaveForwardAsset {
        forward_id: None,
        name: "本地数据库".to_owned(),
        kind: "Local".to_owned(),
        bind_host: "127.0.0.1".to_owned(),
        bind_port: "15432".to_owned(),
        target_host: "db.internal".to_owned(),
        target_port: "5432".to_owned(),
        tags: "db".to_owned(),
        auto_start: false,
        exit_on_failure: true,
    });

    assert!(created.changed());
    assert_eq!(state.storage.forward_asset_count(), 1);
    let forward_id = state.storage.forward_assets[0].id;
    assert!(state.storage.forward_assets[0].exit_on_failure);

    let updated = state.apply(Message::SaveForwardAsset {
        forward_id: Some(forward_id),
        name: "动态代理".to_owned(),
        kind: "Dynamic".to_owned(),
        bind_host: "127.0.0.1".to_owned(),
        bind_port: "1080".to_owned(),
        target_host: String::new(),
        target_port: String::new(),
        tags: "proxy".to_owned(),
        auto_start: false,
        exit_on_failure: false,
    });

    assert!(updated.changed());
    assert_eq!(state.storage.forward_asset_count(), 1);
    assert_eq!(state.storage.forward_assets[0].name, "动态代理");

    let removed = state.apply(Message::RemoveForwardAsset { forward_id });
    assert!(removed.changed());
    assert_eq!(state.storage.forward_asset_count(), 0);
}
