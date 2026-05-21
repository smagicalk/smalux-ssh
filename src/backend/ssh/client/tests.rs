use super::*;
use std::time::Duration;

use russh::keys::PublicKey;

use super::handler::SharedHostKeyResult;
use super::settings::test_constants::{
    DEFAULT_INACTIVITY_TIMEOUT_SECS, DEFAULT_KEEPALIVE_INTERVAL_SECS, DEFAULT_KEEPALIVE_MAX,
};
use crate::backend::BackendExecutionError;

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
