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
    agent_identity_authentication_error, agent_identity_error, authentication_error,
    authentication_rejected_error, decode_private_key, select_agent_identity,
};
pub use channel::{
    ChannelRequestStatus, EXEC_COMMAND_OPERATION, OPEN_SHELL_OPERATION,
    OPEN_SHELL_SESSION_OPERATION, REQUEST_SHELL_OPERATION, RUN_COMMAND_OPERATION,
    RUN_COMMAND_SESSION_OPERATION, SEND_SHELL_INPUT_OPERATION, SFTP_OPERATION, SHELL_EOF_OPERATION,
    SHELL_INPUT_OPERATION, SHELL_RESIZE_OPERATION, START_TUNNEL_OPERATION, authenticated_event,
    authenticating_event, channel_error, channel_request_ended_error,
    collect_channel_request_message, collect_command_message, command_exited_event,
    connected_event, connected_session_error, connecting_event, connection_error,
    disconnected_event, exit_status_to_i32, host_key_rejected_error, host_key_verified_event,
    is_channel_failure, output_event, pty_columns, pty_rows, remote_command_started_event,
    shell_drain_should_stop, shell_message_to_event, shell_opened_event,
};
pub use handler::{
    ForwardedChannel, SharedForwardedChannels, SharedHostKeyResult, SshClientHandler,
};
pub use host_key::{
    HostKeyCheck, HostKeyPolicy, host_key_algorithm, host_key_fingerprint,
    host_key_policy_for_known_hosts,
};
pub use settings::{
    DEFAULT_INACTIVITY_TIMEOUT_SECS, DEFAULT_KEEPALIVE_INTERVAL_SECS, DEFAULT_KEEPALIVE_MAX,
    RusshClientSettings,
};
pub use sftp::{
    CLOSE_SFTP_OPERATION, CREATE_DIR_OPERATION, DOWNLOAD_CLOSE_REMOTE_OPERATION,
    DOWNLOAD_FLUSH_LOCAL_OPERATION, DOWNLOAD_OPEN_LOCAL_OPERATION, DOWNLOAD_OPEN_REMOTE_OPERATION,
    DOWNLOAD_READ_REMOTE_OPERATION, DOWNLOAD_STAT_REMOTE_OPERATION, DOWNLOAD_WRITE_LOCAL_OPERATION,
    LIST_DIR_OPERATION, OPEN_SFTP_OPERATION, OPEN_SFTP_SESSION_OPERATION, REMOVE_FILE_OPERATION,
    REQUEST_SFTP_OPERATION, SFTP_SUBSYSTEM_NAME, UPLOAD_CLOSE_REMOTE_OPERATION,
    UPLOAD_OPEN_LOCAL_OPERATION, UPLOAD_OPEN_REMOTE_OPERATION, UPLOAD_READ_LOCAL_OPERATION,
    UPLOAD_STAT_LOCAL_OPERATION, UPLOAD_WRITE_REMOTE_OPERATION, copy_transfer_with_progress,
    is_sftp_failure, join_remote_path, parent_remote_dir, sftp_entries_event,
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
