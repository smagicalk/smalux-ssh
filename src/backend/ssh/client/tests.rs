use super::*;
use std::io;
use std::time::Duration;

use russh::client;
use russh::keys::PublicKey;

use super::auth::{decode_private_key, select_agent_identity};
use super::handler::{SharedForwardedChannels, SharedHostKeyResult};
use super::host_key::host_key_fingerprint;
use super::settings::test_constants::{
    DEFAULT_INACTIVITY_TIMEOUT_SECS, DEFAULT_KEEPALIVE_INTERVAL_SECS, DEFAULT_KEEPALIVE_MAX,
};
use crate::backend::BackendExecutionError;
use crate::backend::ssh::{SshAuthPlan, SshConnectionPlan};
use crate::model::{HostKeyVerification, KeyAlgorithm, KnownHostEntry};

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

#[test]
fn agent_identity_can_be_selected_by_algorithm_hint_and_missing_match_returns_none() {
    let key = sample_public_key();
    let identities = vec![key.clone()];

    let by_algorithm = select_agent_identity(&identities, Some("Ed25519"));
    let missing = select_agent_identity(&identities, Some("RSA"));

    assert_eq!(by_algorithm, Some(key));
    assert_eq!(missing, None);
}

#[test]
fn authentication_error_preserves_username_and_reason() {
    let error = super::auth::authentication_error("deploy", io::Error::other("boom"));

    assert!(matches!(
        error,
        BackendExecutionError::AuthenticationFailed {
            username,
            reason,
        } if username == "deploy" && reason == "boom"
    ));
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
        SharedForwardedChannels::default(),
    );

    let accepted = client::Handler::check_server_key(&mut handler, &key)
        .await
        .expect("主机密钥检查不应失败");

    assert!(accepted);
    let check = shared.get().expect("处理器应该记录主机密钥检查结果");
    assert_eq!(check.host, "example.com");
    assert_eq!(check.port, 22);
    assert_eq!(check.key_algorithm, KeyAlgorithm::Ed25519);
    assert_eq!(check.verification, HostKeyVerification::Unknown);
}

#[tokio::test]
async fn handler_records_rejected_host_key_verification_result() {
    let key = sample_public_key();
    let shared = SharedHostKeyResult::default();
    let mut handler = SshClientHandler::new(
        "example.com".to_owned(),
        2222,
        HostKeyPolicy::default(),
        shared.clone(),
        SharedForwardedChannels::default(),
    );

    let accepted = client::Handler::check_server_key(&mut handler, &key)
        .await
        .expect("主机密钥拒绝结果也应该正常返回");

    assert!(!accepted);
    let check = shared.get().expect("处理器应该记录被拒绝的主机密钥结果");
    assert!(!check.accepted);
    assert_eq!(check.host, "example.com");
    assert_eq!(check.port, 2222);
    assert_eq!(check.verification, HostKeyVerification::Unknown);
}

#[tokio::test]
async fn forwarded_channel_subscription_replaces_matching_endpoint() {
    let shared = SharedForwardedChannels::default();
    let mut stale_receiver = shared.subscribe("127.0.0.1", 8022);
    let _current_receiver = shared.subscribe("127.0.0.1", 8022);

    let stale_closed = tokio::time::timeout(Duration::from_millis(50), stale_receiver.recv())
        .await
        .expect("被替换的 forwarded channel 订阅应该立即关闭");

    assert!(stale_closed.is_none());
}

#[test]
fn host_key_or_connection_error_prefers_rejected_host_key() {
    let key = sample_public_key();
    let check = HostKeyPolicy::default().check("example.com", 22, &key);
    let expected = check.clone();
    let shared = SharedHostKeyResult::default();
    shared.set(check);

    let error =
        super::host_key_or_connection_error("example.com:22", &shared, russh::Error::Inconsistent);

    assert_eq!(
        error,
        BackendExecutionError::HostKeyRejected {
            host: expected.host,
            port: expected.port,
            key_algorithm: expected.key_algorithm,
            fingerprint: expected.fingerprint,
            verification: expected.verification,
        }
    );
}

#[test]
fn host_key_or_connection_error_returns_connection_failure_without_rejected_host_key() {
    let shared = SharedHostKeyResult::default();

    let error =
        super::host_key_or_connection_error("example.com:22", &shared, russh::Error::Inconsistent);

    assert!(matches!(
        error,
        BackendExecutionError::ConnectionFailed {
            endpoint,
            reason,
        } if endpoint == "example.com:22" && !reason.is_empty()
    ));
}

#[test]
fn host_key_policy_for_plan_prefers_plan_known_hosts_over_configured_known_hosts() {
    let connector = RusshConnector::with_settings(RusshClientSettings::default())
        .with_host_key_policy(HostKeyPolicy::KnownHosts(vec![KnownHostEntry {
            host: "configured.example.com".to_owned(),
            port: 22,
            key_algorithm: KeyAlgorithm::Ed25519,
            fingerprint: "SHA256:configured".to_owned(),
            trusted: true,
        }]));
    let plan_known_hosts = vec![KnownHostEntry {
        host: "example.com".to_owned(),
        port: 22,
        key_algorithm: KeyAlgorithm::Ed25519,
        fingerprint: "SHA256:plan".to_owned(),
        trusted: true,
    }];
    let plan_with_known_hosts = SshConnectionPlan {
        host: "example.com".to_owned(),
        port: 22,
        endpoint: "example.com:22".to_owned(),
        auth: SshAuthPlan::Agent {
            username: "deploy".to_owned(),
            key_hint: None,
        },
        known_hosts: plan_known_hosts.clone(),
    };
    let plan_without_known_hosts = SshConnectionPlan {
        known_hosts: Vec::new(),
        ..plan_with_known_hosts.clone()
    };

    assert_eq!(
        connector.host_key_policy_for_plan(&plan_with_known_hosts),
        HostKeyPolicy::KnownHosts(plan_known_hosts)
    );
    assert_eq!(
        connector.host_key_policy_for_plan(&plan_without_known_hosts),
        HostKeyPolicy::KnownHosts(vec![KnownHostEntry {
            host: "configured.example.com".to_owned(),
            port: 22,
            key_algorithm: KeyAlgorithm::Ed25519,
            fingerprint: "SHA256:configured".to_owned(),
            trusted: true,
        }])
    );
}
