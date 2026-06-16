use super::*;

#[test]
fn backend_queue_pump_records_unknown_host_key_candidate() {
    let mut state = CoreState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.apply(Message::OpenShell { host_id });
    let mut executor = RejectingHostKeyExecutor::new(HostKeyVerification::Unknown);

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(outcome.applied_backend_events, 1);
    assert!(state.backend_commands.is_empty());
    assert_eq!(state.storage.known_host_count(), 1);
    assert_eq!(
        state.storage.known_hosts[0],
        KnownHostEntry::untrusted("example.com", 22, KeyAlgorithm::Ed25519, "SHA256:new")
    );
    assert!(matches!(
        &state.sessions.tabs[0].status,
        SessionStatus::Failed { reason } if reason.contains("主机密钥未被信任")
    ));
}

#[test]
fn backend_queue_pump_does_not_overwrite_trusted_host_on_mismatch() {
    let mut state = CoreState::default();
    let host = sample_host();
    let host_id = host.id;
    state.storage.upsert_host(host);
    state.storage.upsert_known_host(KnownHostEntry {
        host: "example.com".to_owned(),
        port: 22,
        key_algorithm: KeyAlgorithm::Ed25519,
        fingerprint: "SHA256:old".to_owned(),
        trusted: true,
    });
    state.apply(Message::OpenShell { host_id });
    let mut executor = RejectingHostKeyExecutor::new(HostKeyVerification::Mismatch {
        expected: "SHA256:old".to_owned(),
        actual: "SHA256:new".to_owned(),
    });

    let outcome = state.drain_backend_queue(&mut executor);

    assert!(outcome.changed());
    assert_eq!(state.storage.known_host_count(), 1);
    assert_eq!(state.storage.known_hosts[0].fingerprint, "SHA256:old");
    assert!(state.storage.known_hosts[0].trusted);
    assert!(state.backend_commands.is_empty());
}
