use super::*;
use std::time::Duration;

use super::settings::test_constants::{
    DEFAULT_INACTIVITY_TIMEOUT_SECS, DEFAULT_KEEPALIVE_INTERVAL_SECS, DEFAULT_KEEPALIVE_MAX,
};

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
