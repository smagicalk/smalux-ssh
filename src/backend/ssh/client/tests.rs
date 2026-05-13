use super::*;
use crate::model::KeyAlgorithm;

fn sample_public_key() -> PublicKey {
    russh::keys::parse_public_key_base64(
        "AAAAC3NzaC1lZDI1NTE5AAAAILagOJFgwaMNhBWQINinKOXmqS4Gh5NgxgriXwdOoINJ",
    )
    .expect("测试公钥应该可以解析")
}

#[test]
fn russh_settings_build_expected_client_config() {
    let settings = RusshClientSettings::default();

    let config = settings.to_russh_config();

    assert_eq!(
        config.inactivity_timeout,
        Some(Duration::from_secs(DEFAULT_INACTIVITY_TIMEOUT_SECS))
    );
    assert_eq!(
        config.keepalive_interval,
        Some(Duration::from_secs(DEFAULT_KEEPALIVE_INTERVAL_SECS))
    );
    assert_eq!(config.keepalive_max, DEFAULT_KEEPALIVE_MAX);
    assert!(config.nodelay);
}

#[test]
fn accept_any_policy_allows_unknown_key_explicitly() {
    let key = sample_public_key();

    let check = HostKeyPolicy::AcceptAny.check("example.com", 22, &key);

    assert!(check.accepted);
    assert_eq!(check.verification, HostKeyVerification::Unknown);
    assert!(check.fingerprint.starts_with("SHA256:"));
}

#[test]
fn known_hosts_policy_accepts_trusted_fingerprint() {
    let key = sample_public_key();
    let fingerprint = host_key_fingerprint(&key);
    let policy = HostKeyPolicy::KnownHosts(vec![KnownHostEntry {
        host: "example.com".to_owned(),
        port: 22,
        key_algorithm: KeyAlgorithm::Ed25519,
        fingerprint: fingerprint.clone(),
        trusted: true,
    }]);

    let check = policy.check("example.com", 22, &key);

    assert!(check.accepted);
    assert_eq!(check.verification, HostKeyVerification::Trusted);
    assert_eq!(check.fingerprint, fingerprint);
}

#[test]
fn known_hosts_policy_rejects_mismatch() {
    let key = sample_public_key();
    let policy = HostKeyPolicy::KnownHosts(vec![KnownHostEntry {
        host: "example.com".to_owned(),
        port: 22,
        key_algorithm: KeyAlgorithm::Ed25519,
        fingerprint: "SHA256:old".to_owned(),
        trusted: true,
    }]);

    let check = policy.check("example.com", 22, &key);

    assert!(!check.accepted);
    assert!(matches!(
        check.verification,
        HostKeyVerification::Mismatch { .. }
    ));
}

#[test]
fn invalid_private_key_maps_to_authentication_failure() {
    let error = decode_private_key("not a private key", None, "deploy")
        .expect_err("非法私钥应该映射为认证失败");

    assert!(matches!(
        error,
        BackendExecutionError::AuthenticationFailed {
            username,
            reason,
        } if username == "deploy" && !reason.is_empty()
    ));
}

#[test]
fn agent_identity_can_be_selected_by_fingerprint_and_comment() {
    let mut key = sample_public_key();
    let fingerprint = host_key_fingerprint(&key);
    key.set_comment("deploy-key");
    let identities = vec![key.clone()];

    let by_fingerprint = select_agent_identity(&identities, Some(&fingerprint));
    let by_comment = select_agent_identity(&identities, Some("deploy"));
    let first = select_agent_identity(&identities, None);

    assert_eq!(by_fingerprint, Some(key.clone()));
    assert_eq!(by_comment, Some(key.clone()));
    assert_eq!(first, Some(key));
}

#[tokio::test]
async fn handler_records_host_key_verification_result() {
    let key = sample_public_key();
    let shared = SharedHostKeyResult::default();
    let mut handler = SshClientHandler::new(
        "example.com".to_owned(),
        22,
        HostKeyPolicy::AcceptAny,
        shared.clone(),
    );

    let accepted = client::Handler::check_server_key(&mut handler, &key)
        .await
        .expect("主机密钥检查不应失败");

    assert!(accepted);
    assert_eq!(shared.get(), Some(HostKeyVerification::Unknown));
}
