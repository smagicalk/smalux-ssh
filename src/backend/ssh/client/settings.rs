//! `russh` 客户端配置兼容导出。

pub use smagical_ssh_client_core::RusshClientSettings;

#[cfg(test)]
pub(super) mod test_constants {
    pub(crate) use smagical_ssh_client_core::{
        DEFAULT_INACTIVITY_TIMEOUT_SECS, DEFAULT_KEEPALIVE_INTERVAL_SECS, DEFAULT_KEEPALIVE_MAX,
    };
}
