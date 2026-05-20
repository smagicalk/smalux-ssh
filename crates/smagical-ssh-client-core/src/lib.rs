//! `russh` SSH 客户端核心组件。

mod agent;
mod channel;
mod handler;
mod host_key;
mod settings;
mod sftp;
mod socks5;
mod tunnel;

pub use agent::{
    agent_identity_error, authentication_error, authentication_rejected_error, decode_private_key,
    select_agent_identity,
};
pub use channel::{
    ChannelRequestStatus, authenticated_event, authenticating_event, channel_error,
    channel_request_ended_error, collect_channel_request_message, collect_command_message,
    command_exited_event, connected_event, connecting_event, disconnected_event,
    exit_status_to_i32, host_key_verified_event, output_event, pty_columns, pty_rows,
    remote_command_started_event, shell_message_to_event, shell_opened_event,
};
pub use handler::{
    ForwardedChannel, SharedForwardedChannels, SharedHostKeyResult, SshClientHandler,
};
pub use host_key::{HostKeyCheck, HostKeyPolicy, host_key_algorithm, host_key_fingerprint};
pub use settings::{
    DEFAULT_INACTIVITY_TIMEOUT_SECS, DEFAULT_KEEPALIVE_INTERVAL_SECS, DEFAULT_KEEPALIVE_MAX,
    RusshClientSettings,
};
pub use sftp::{
    copy_transfer_with_progress, join_remote_path, parent_remote_dir, sftp_entries_event,
    sftp_entry_from_parts, sftp_error, sftp_io_error, transfer_event,
};
pub use socks5::{Socks5Target, read_socks5_target, write_socks5_success};
pub use tunnel::{
    DIRECT_TCPIP_OPERATION, DIRECT_TCPIP_RULE_NAME, DYNAMIC_SOCKS5_OPERATION,
    DYNAMIC_SOCKS5_RULE_NAME, REMOTE_FORWARD_RULE_NAME, RemoteTunnel, TUNNEL_ACCEPT_TICK,
    copy_bidirectional, remote_tunnel, tunnel_error, tunnel_io_error, tunnel_reason_error,
    tunnel_running_event, tunnel_status_event, tunnel_stopped_event,
};

#[cfg(test)]
mod tests;
