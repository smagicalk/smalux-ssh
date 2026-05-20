//! `russh` SSH 客户端核心组件。

mod agent;
mod channel;
mod handler;
mod host_key;
mod settings;
mod sftp;

pub use agent::select_agent_identity;
pub use channel::{
    collect_command_message, exit_status_to_i32, output_event, shell_message_to_event,
};
pub use handler::{
    ForwardedChannel, SharedForwardedChannels, SharedHostKeyResult, SshClientHandler,
};
pub use host_key::{HostKeyCheck, HostKeyPolicy, host_key_algorithm, host_key_fingerprint};
pub use settings::{
    DEFAULT_INACTIVITY_TIMEOUT_SECS, DEFAULT_KEEPALIVE_INTERVAL_SECS, DEFAULT_KEEPALIVE_MAX,
    RusshClientSettings,
};
pub use sftp::{join_remote_path, parent_remote_dir, sftp_entry_from_parts, transfer_event};

#[cfg(test)]
mod tests;
