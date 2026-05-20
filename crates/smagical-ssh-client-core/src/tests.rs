use super::*;

#[test]
fn default_settings_use_expected_timeouts() {
    let settings = RusshClientSettings::default();

    assert_eq!(
        settings.inactivity_timeout,
        Duration::from_secs(DEFAULT_INACTIVITY_TIMEOUT_SECS)
    );
    assert_eq!(
        settings.keepalive_interval,
        Some(Duration::from_secs(DEFAULT_KEEPALIVE_INTERVAL_SECS))
    );
    assert_eq!(settings.keepalive_max, DEFAULT_KEEPALIVE_MAX);
    assert!(settings.nodelay);
}

#[test]
fn settings_build_expected_russh_config() {
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
fn custom_settings_build_russh_config_without_keepalive() {
    let settings = RusshClientSettings {
        inactivity_timeout: Duration::from_secs(5),
        keepalive_interval: None,
        keepalive_max: 0,
        nodelay: false,
    };

    let config = settings.to_russh_config();

    assert_eq!(config.inactivity_timeout, Some(Duration::from_secs(5)));
    assert_eq!(config.keepalive_interval, None);
    assert_eq!(config.keepalive_max, 0);
    assert!(!config.nodelay);
}
