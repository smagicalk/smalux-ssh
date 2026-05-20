//! `russh` 客户端配置。

use std::time::Duration;

use russh::client;

pub const DEFAULT_INACTIVITY_TIMEOUT_SECS: u64 = 30;
pub const DEFAULT_KEEPALIVE_INTERVAL_SECS: u64 = 15;
pub const DEFAULT_KEEPALIVE_MAX: usize = 3;

/// `russh` 客户端配置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RusshClientSettings {
    pub inactivity_timeout: Duration,
    pub keepalive_interval: Option<Duration>,
    pub keepalive_max: usize,
    pub nodelay: bool,
}

impl Default for RusshClientSettings {
    fn default() -> Self {
        Self {
            inactivity_timeout: Duration::from_secs(DEFAULT_INACTIVITY_TIMEOUT_SECS),
            keepalive_interval: Some(Duration::from_secs(DEFAULT_KEEPALIVE_INTERVAL_SECS)),
            keepalive_max: DEFAULT_KEEPALIVE_MAX,
            nodelay: true,
        }
    }
}

impl RusshClientSettings {
    /// 转换为 `russh` 原生客户端配置。
    pub fn to_russh_config(&self) -> client::Config {
        client::Config {
            inactivity_timeout: Some(self.inactivity_timeout),
            keepalive_interval: self.keepalive_interval,
            keepalive_max: self.keepalive_max,
            nodelay: self.nodelay,
            ..Default::default()
        }
    }
}
