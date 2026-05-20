//! `russh` SSH 客户端核心组件。

mod agent;
mod handler;
mod host_key;
mod settings;

pub use agent::select_agent_identity;
pub use handler::{
    ForwardedChannel, SharedForwardedChannels, SharedHostKeyResult, SshClientHandler,
};
pub use host_key::{HostKeyCheck, HostKeyPolicy, host_key_algorithm, host_key_fingerprint};
pub use settings::{
    DEFAULT_INACTIVITY_TIMEOUT_SECS, DEFAULT_KEEPALIVE_INTERVAL_SECS, DEFAULT_KEEPALIVE_MAX,
    RusshClientSettings,
};

#[cfg(test)]
mod tests;
