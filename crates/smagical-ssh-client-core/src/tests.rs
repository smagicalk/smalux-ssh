use super::*;
use russh::client;
use std::io::{Cursor, Error, ErrorKind};
use std::pin::Pin;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};

use russh::ChannelMsg;
use russh::CryptoVec;
use russh::Sig;
use russh::keys::PublicKey;
use russh_sftp::protocol::FileAttributes;
use smagical_backend_core::{BackendEvent, BackendExecutionError};
use smagical_core::{
    HostKeyVerification, KeyAlgorithm, KnownHostEntry, SftpEntry, SftpEntryKind, TransferId,
    TransferStatus, TunnelStatus,
};
use smagical_terminal::TerminalSize;
use uuid::Uuid;

const TEST_SFTP_TRANSFER_CHUNK_SIZE: usize = 64 * 1024;

fn sample_public_key() -> PublicKey {
    russh::keys::parse_public_key_base64(
        "AAAAC3NzaC1lZDI1NTE5AAAAILagOJFgwaMNhBWQINinKOXmqS4Gh5NgxgriXwdOoINJ",
    )
    .expect("测试公钥应该可以解析")
}

fn session_id() -> smagical_core::SessionId {
    smagical_core::SessionId(Uuid::new_v4())
}

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

#[test]
fn accept_any_policy_allows_unknown_key_explicitly() {
    let key = sample_public_key();

    let check = HostKeyPolicy::AcceptAny.check("example.com", 22, &key);

    assert!(check.accepted);
    assert_eq!(check.host, "example.com");
    assert_eq!(check.port, 22);
    assert_eq!(check.key_algorithm, KeyAlgorithm::Ed25519);
    assert_eq!(check.verification, HostKeyVerification::Unknown);
    assert!(check.fingerprint.starts_with("SHA256:"));
}

#[test]
fn default_policy_rejects_unknown_key_for_first_trust() {
    let key = sample_public_key();

    let check = HostKeyPolicy::default().check("example.com", 22, &key);

    assert!(!check.accepted);
    assert_eq!(check.verification, HostKeyVerification::Unknown);
    assert_eq!(check.key_algorithm, KeyAlgorithm::Ed25519);
}

#[test]
fn known_hosts_policy_accepts_trusted_fingerprint() {
    let key = sample_public_key();
    let fingerprint = host_key_fingerprint(&key);
    let policy = HostKeyPolicy::KnownHosts(vec![KnownHostEntry {
        host: "example.com".to_owned(),
        port: 22,
        key_algorithm: KeyAlgorithm::Ed25519,
        fingerprint: fingerprint.clone(),
        trusted: true,
    }]);

    let check = policy.check("example.com", 22, &key);

    assert!(check.accepted);
    assert_eq!(check.verification, HostKeyVerification::Trusted);
    assert_eq!(check.fingerprint, fingerprint);
}

#[test]
fn known_hosts_policy_rejects_mismatch() {
    let key = sample_public_key();
    let policy = HostKeyPolicy::KnownHosts(vec![KnownHostEntry {
        host: "example.com".to_owned(),
        port: 22,
        key_algorithm: KeyAlgorithm::Ed25519,
        fingerprint: "SHA256:old".to_owned(),
        trusted: true,
    }]);

    let check = policy.check("example.com", 22, &key);

    assert!(!check.accepted);
    assert!(matches!(
        check.verification,
        HostKeyVerification::Mismatch { .. }
    ));
}

#[test]
fn known_hosts_policy_treats_unmatched_host_or_port_as_unknown() {
    let key = sample_public_key();
    let policy = HostKeyPolicy::KnownHosts(vec![KnownHostEntry {
        host: "example.com".to_owned(),
        port: 22,
        key_algorithm: KeyAlgorithm::Ed25519,
        fingerprint: host_key_fingerprint(&key),
        trusted: true,
    }]);

    let check = policy.check("example.com", 2222, &key);

    assert!(!check.accepted);
    assert_eq!(check.verification, HostKeyVerification::Unknown);
}

#[test]
fn host_key_algorithm_maps_public_key_algorithm() {
    let key = sample_public_key();

    assert_eq!(host_key_algorithm(&key), KeyAlgorithm::Ed25519);
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
fn authentication_error_preserves_username_and_reason() {
    let error = authentication_error("deploy", std::io::Error::other("boom"));

    assert!(matches!(
        error,
        BackendExecutionError::AuthenticationFailed {
            username,
            reason,
        } if username == "deploy" && reason == "boom"
    ));
}

#[test]
fn authentication_rejected_error_reports_method() {
    let error = authentication_rejected_error("deploy", "password");

    assert!(matches!(
        error,
        BackendExecutionError::AuthenticationFailed {
            username,
            reason,
        } if username == "deploy" && reason == "password 认证被服务器拒绝"
    ));
}

#[test]
fn agent_identity_error_reports_hint_or_empty_agent() {
    assert_eq!(
        agent_identity_error(Some("deploy-key")),
        "ssh-agent 中没有匹配的身份：deploy-key"
    );
    assert_eq!(agent_identity_error(None), "ssh-agent 中没有可用身份");
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
fn sftp_entry_mapping_preserves_path_kind_and_metadata() {
    let mut metadata = FileAttributes::empty();
    metadata.size = Some(4096);
    metadata.mtime = Some(1_700_000_000);
    metadata.permissions = Some(0o100644);

    let entry = sftp_entry_from_parts("/var/log", "syslog".to_owned(), metadata);

    assert_eq!(entry.name, "syslog");
    assert_eq!(entry.remote_path, "/var/log/syslog");
    assert_eq!(entry.kind, SftpEntryKind::File);
    assert_eq!(entry.size, Some(4096));
    assert_eq!(entry.modified_at_unix_secs, Some(1_700_000_000));
    assert_eq!(entry.permissions, Some(0o100644));
}

#[test]
fn sftp_path_helpers_handle_root_and_nested_paths() {
    assert_eq!(join_remote_path("/", "etc"), "/etc");
    assert_eq!(join_remote_path("/var/log/", "syslog"), "/var/log/syslog");
    assert_eq!(parent_remote_dir("/var/log/syslog"), "/var/log");
    assert_eq!(parent_remote_dir("/tmp"), "/");
    assert_eq!(parent_remote_dir("/"), "/");
}

#[test]
fn sftp_entries_event_preserves_path_and_entries() {
    let session_id = session_id();
    let entries = vec![SftpEntry {
        remote_path: "/var/log/syslog".to_owned(),
        name: "syslog".to_owned(),
        kind: SftpEntryKind::File,
        size: Some(4096),
        modified_at_unix_secs: Some(1_700_000_000),
        permissions: Some(0o100644),
    }];

    let event = sftp_entries_event(session_id, "/var/log".to_owned(), entries.clone());

    assert_eq!(
        event,
        BackendEvent::SftpEntries {
            session_id,
            remote_path: "/var/log".to_owned(),
            entries,
        }
    );
}

#[test]
fn sftp_transfer_event_carries_total_and_progress_bytes() {
    let session_id = session_id();
    let transfer_id = TransferId(Uuid::new_v4());

    let event = transfer_event(
        session_id,
        transfer_id,
        Some(4096),
        2048,
        TransferStatus::Running,
    );

    assert_eq!(
        event,
        BackendEvent::TransferProgress {
            session_id,
            transfer_id,
            total_bytes: Some(4096),
            transferred_bytes: 2048,
            status: TransferStatus::Running,
        }
    );
}

#[test]
fn pty_dimensions_are_never_zero() {
    let size = TerminalSize {
        columns: 0,
        rows: 0,
    };

    assert_eq!(pty_columns(size), 1);
    assert_eq!(pty_rows(size), 1);
}

#[test]
fn ssh_error_helpers_preserve_operation_and_reason() {
    let channel = channel_error("open shell", russh::Error::Inconsistent);
    let missing_session = connected_session_error("run command");
    let connection = connection_error("example.com:22", russh::Error::Inconsistent);
    let rejected = host_key_rejected_error(HostKeyCheck {
        host: "example.com".to_owned(),
        port: 2222,
        key_algorithm: KeyAlgorithm::Ed25519,
        verification: HostKeyVerification::Mismatch {
            expected: "SHA256:old".to_owned(),
            actual: "SHA256:test".to_owned(),
        },
        accepted: false,
        fingerprint: "SHA256:test".to_owned(),
    });
    let sftp = sftp_error("list dir", "permission denied");
    let io = sftp_io_error(
        "upload local",
        std::io::Error::new(std::io::ErrorKind::Other, "disk full"),
    );
    let tunnel = tunnel_error("proxy", russh::Error::Inconsistent);
    let tunnel_io = tunnel_io_error("local-forward", std::io::Error::other("bind failed"));
    let tunnel_reason = tunnel_reason_error("dynamic-socks5", "missing no-auth method");

    assert!(is_channel_failure(&channel));
    assert!(!is_sftp_failure(&channel));
    assert!(is_channel_failure(&missing_session));
    assert!(!is_channel_failure(&connection));
    assert!(is_sftp_failure(&sftp));
    assert!(is_sftp_failure(&io));

    assert!(matches!(
        channel,
        BackendExecutionError::ChannelFailed {
            operation,
            reason,
        } if operation == "open shell" && !reason.is_empty()
    ));
    assert!(matches!(
        missing_session,
        BackendExecutionError::ChannelFailed {
            operation,
            reason,
        } if operation == "run command" && reason == "session is not connected"
    ));
    assert!(matches!(
        connection,
        BackendExecutionError::ConnectionFailed {
            endpoint,
            reason,
        } if endpoint == "example.com:22" && !reason.is_empty()
    ));
    assert!(matches!(
        rejected,
        BackendExecutionError::HostKeyRejected {
            host,
            port,
            key_algorithm,
            fingerprint,
            verification,
        } if host == "example.com"
            && port == 2222
            && key_algorithm == KeyAlgorithm::Ed25519
            && fingerprint == "SHA256:test"
            && verification == HostKeyVerification::Mismatch {
                expected: "SHA256:old".to_owned(),
                actual: "SHA256:test".to_owned(),
            }
    ));
    assert!(matches!(
        sftp,
        BackendExecutionError::SftpFailed {
            operation,
            reason,
        } if operation == "list dir" && reason == "permission denied"
    ));
    assert!(matches!(
        io,
        BackendExecutionError::SftpFailed {
            operation,
            reason,
        } if operation == "upload local" && reason.contains("disk full")
    ));
    assert!(matches!(
        tunnel,
        BackendExecutionError::TunnelFailed { rule_name, reason }
            if rule_name == "proxy" && !reason.is_empty()
    ));
    assert!(matches!(
        tunnel_io,
        BackendExecutionError::TunnelFailed { rule_name, reason }
            if rule_name == "local-forward" && reason == "bind failed"
    ));
    assert!(matches!(
        tunnel_reason,
        BackendExecutionError::TunnelFailed { rule_name, reason }
            if rule_name == "dynamic-socks5" && reason == "missing no-auth method"
    ));
}

#[test]
fn remote_tunnel_reports_endpoint_and_can_stop() {
    let session_id = session_id();
    let running = Arc::new(AtomicBool::new(true));
    let tunnel = remote_tunnel(
        session_id,
        "proxy".to_owned(),
        running.clone(),
        "127.0.0.1".to_owned(),
        1080,
    );

    assert_eq!(tunnel.session_id(), session_id);
    assert_eq!(tunnel.rule_name(), "proxy");
    assert_eq!(tunnel.bind_endpoint(), "127.0.0.1:1080");
    tunnel.stop();
    assert!(!running.load(Ordering::SeqCst));
}

#[test]
fn tunnel_internal_names_are_stable() {
    assert_eq!(DIRECT_TCPIP_OPERATION, "direct tcpip");
    assert_eq!(DYNAMIC_SOCKS5_OPERATION, "dynamic socks5");
    assert_eq!(DIRECT_TCPIP_RULE_NAME, "direct-tcpip");
    assert_eq!(REMOTE_FORWARD_RULE_NAME, "remote-forward");
    assert_eq!(DYNAMIC_SOCKS5_RULE_NAME, "dynamic-socks5");
    assert_eq!(TUNNEL_ACCEPT_TICK, Duration::from_millis(250));
}

#[test]
fn tunnel_status_events_preserve_session_rule_and_status() {
    let session_id = session_id();

    assert_eq!(
        tunnel_running_event(session_id, "proxy".to_owned()),
        BackendEvent::TunnelStatusChanged {
            session_id,
            rule_name: "proxy".to_owned(),
            status: TunnelStatus::Running,
        }
    );
    assert_eq!(
        tunnel_status_event(session_id, "proxy".to_owned(), TunnelStatus::Stopped),
        BackendEvent::TunnelStatusChanged {
            session_id,
            rule_name: "proxy".to_owned(),
            status: TunnelStatus::Stopped,
        }
    );
    assert_eq!(
        tunnel_stopped_event(session_id, "proxy".to_owned()),
        BackendEvent::TunnelStatusChanged {
            session_id,
            rule_name: "proxy".to_owned(),
            status: TunnelStatus::Stopped,
        }
    );
}

#[tokio::test]
async fn copy_bidirectional_moves_bytes_in_both_directions() {
    let (mut left_client, mut left_server) = tokio::io::duplex(64);
    let (mut right_client, mut right_server) = tokio::io::duplex(64);

    let pipe = tokio::spawn(async move {
        copy_bidirectional(&mut left_server, &mut right_server)
            .await
            .expect("双向复制应该成功");
    });

    left_client.write_all(b"left-to-right").await.unwrap();
    right_client.write_all(b"right-to-left").await.unwrap();
    let mut from_left = vec![0_u8; b"left-to-right".len()];
    let mut from_right = vec![0_u8; b"right-to-left".len()];
    right_client.read_exact(&mut from_left).await.unwrap();
    left_client.read_exact(&mut from_right).await.unwrap();

    assert_eq!(from_left, b"left-to-right");
    assert_eq!(from_right, b"right-to-left");

    drop(left_client);
    drop(right_client);
    pipe.await.unwrap();
}

fn transfer_id() -> TransferId {
    TransferId(Uuid::new_v4())
}

#[tokio::test]
async fn copy_transfer_with_progress_emits_chunk_progress() {
    let session_id = session_id();
    let transfer_id = transfer_id();
    let mut reader = Cursor::new(vec![7_u8; TEST_SFTP_TRANSFER_CHUNK_SIZE + 3]);
    let mut writer = Vec::new();

    let (transferred, events) = copy_transfer_with_progress(
        session_id,
        transfer_id,
        Some((TEST_SFTP_TRANSFER_CHUNK_SIZE + 3) as u64),
        &mut reader,
        &mut writer,
        "read transfer",
        "write transfer",
    )
    .await
    .unwrap();

    assert_eq!(transferred, (TEST_SFTP_TRANSFER_CHUNK_SIZE + 3) as u64);
    assert_eq!(writer, vec![7_u8; TEST_SFTP_TRANSFER_CHUNK_SIZE + 3]);
    assert_eq!(
        events,
        vec![
            transfer_event(
                session_id,
                transfer_id,
                Some((TEST_SFTP_TRANSFER_CHUNK_SIZE + 3) as u64),
                TEST_SFTP_TRANSFER_CHUNK_SIZE as u64,
                TransferStatus::Running,
            ),
            transfer_event(
                session_id,
                transfer_id,
                Some((TEST_SFTP_TRANSFER_CHUNK_SIZE + 3) as u64),
                (TEST_SFTP_TRANSFER_CHUNK_SIZE + 3) as u64,
                TransferStatus::Running,
            ),
        ]
    );
}

#[tokio::test]
async fn copy_transfer_with_progress_handles_empty_stream() {
    let mut reader = Cursor::new(Vec::new());
    let mut writer = Vec::new();

    let (transferred, events) = copy_transfer_with_progress(
        session_id(),
        transfer_id(),
        Some(0),
        &mut reader,
        &mut writer,
        "read empty",
        "write empty",
    )
    .await
    .unwrap();

    assert_eq!(transferred, 0);
    assert!(events.is_empty());
    assert!(writer.is_empty());
}

#[tokio::test]
async fn copy_transfer_with_progress_maps_write_errors() {
    let mut reader = Cursor::new(b"payload".to_vec());
    let mut writer = FailingTransferWriter;

    let error = copy_transfer_with_progress(
        session_id(),
        transfer_id(),
        Some(7),
        &mut reader,
        &mut writer,
        "read payload",
        "write payload",
    )
    .await
    .unwrap_err();

    assert!(matches!(
        error,
        BackendExecutionError::SftpFailed {
            operation,
            reason,
        } if operation == "write payload" && reason.contains("injected write failure")
    ));
}

#[tokio::test]
async fn copy_transfer_with_progress_maps_read_errors() {
    let mut reader = FailingTransferReader;
    let mut writer = Vec::new();

    let error = copy_transfer_with_progress(
        session_id(),
        transfer_id(),
        Some(0),
        &mut reader,
        &mut writer,
        "read payload",
        "write payload",
    )
    .await
    .unwrap_err();

    assert!(matches!(
        error,
        BackendExecutionError::SftpFailed {
            operation,
            reason,
        } if operation == "read payload" && reason.contains("injected read failure")
    ));
}

struct FailingTransferWriter;

struct FailingTransferReader;

impl AsyncWrite for FailingTransferWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        Poll::Ready(Err(Error::new(ErrorKind::Other, "injected write failure")))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}

impl AsyncRead for FailingTransferWriter {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}

impl AsyncRead for FailingTransferReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut ReadBuf<'_>,
    ) -> Poll<Result<(), Error>> {
        Poll::Ready(Err(Error::new(ErrorKind::Other, "injected read failure")))
    }
}

impl AsyncWrite for FailingTransferReader {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        Poll::Ready(Ok(0))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}

async fn parse_socks5_target_from_client_bytes(
    bytes: &'static [u8],
) -> Result<(String, u16, Vec<u8>), String> {
    let (mut client, mut server) = tokio::io::duplex(128);
    let client_task = tokio::spawn(async move {
        client.write_all(bytes).await.unwrap();
        let mut response = [0_u8; 2];
        client.read_exact(&mut response).await.unwrap();
        response.to_vec()
    });

    let target = read_socks5_target(&mut server).await?;
    let response = client_task.await.unwrap();
    Ok((target.host, target.port, response))
}

async fn reject_socks5_greeting_from_client_bytes(bytes: &'static [u8]) -> Result<Vec<u8>, String> {
    let (mut client, mut server) = tokio::io::duplex(128);
    let client_task = tokio::spawn(async move {
        client.write_all(bytes).await.unwrap();
        let mut response = [0_u8; 2];
        client.read_exact(&mut response).await.unwrap();
        response.to_vec()
    });

    let error = match read_socks5_target(&mut server).await {
        Ok(_) => panic!("SOCKS5 greeting without no-auth should be rejected"),
        Err(error) => error,
    };
    let response = client_task.await.unwrap();
    assert_eq!(error, "SOCKS5 no-auth method is required");
    Ok(response)
}

async fn parse_socks5_target_with_error(
    bytes: &'static [u8],
    read_response: bool,
) -> Result<String, String> {
    let (mut client, mut server) = tokio::io::duplex(128);
    let client_task = tokio::spawn(async move {
        client.write_all(bytes).await.unwrap();
        if read_response {
            let mut response = [0_u8; 2];
            client.read_exact(&mut response).await.unwrap();
        }
    });

    let error = match read_socks5_target(&mut server).await {
        Ok(_) => panic!("SOCKS5 target parsing should fail"),
        Err(error) => error,
    };
    let _ = client_task.await.unwrap();
    Ok(error)
}

#[tokio::test]
async fn socks5_target_reads_ipv4_connect_request() {
    let (host, port, response) = parse_socks5_target_from_client_bytes(&[
        0x05, 0x01, 0x00, // greeting: no auth
        0x05, 0x01, 0x00, 0x01, // CONNECT IPv4
        127, 0, 0, 1, 0x1F, 0x90, // 127.0.0.1:8080
    ])
    .await
    .unwrap();

    assert_eq!(host, "127.0.0.1");
    assert_eq!(port, 8080);
    assert_eq!(response, vec![0x05, 0x00]);
}

#[tokio::test]
async fn socks5_target_reads_domain_connect_request() {
    let (host, port, response) = parse_socks5_target_from_client_bytes(&[
        0x05, 0x01, 0x00, // greeting: no auth
        0x05, 0x01, 0x00, 0x03, // CONNECT domain
        11, b'e', b'x', b'a', b'm', b'p', b'l', b'e', b'.', b'c', b'o', b'm', 0x00, 0x16,
    ])
    .await
    .unwrap();

    assert_eq!(host, "example.com");
    assert_eq!(port, 22);
    assert_eq!(response, vec![0x05, 0x00]);
}

#[tokio::test]
async fn socks5_target_reads_ipv6_connect_request() {
    let (host, port, response) = parse_socks5_target_from_client_bytes(&[
        0x05, 0x01, 0x00, // greeting: no auth
        0x05, 0x01, 0x00, 0x04, // CONNECT IPv6
        0x20, 0x01, 0x0D, 0xB8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0x01, 0xBB,
    ])
    .await
    .unwrap();

    assert_eq!(host, "2001:db8::1");
    assert_eq!(port, 443);
    assert_eq!(response, vec![0x05, 0x00]);
}

#[tokio::test]
async fn socks5_target_rejects_clients_without_no_auth_method() {
    let response = reject_socks5_greeting_from_client_bytes(&[
        0x05, 0x01, 0x02, // greeting: username/password only
    ])
    .await
    .unwrap();

    assert_eq!(response, vec![0x05, 0xFF]);
}

#[tokio::test]
async fn socks5_target_rejects_non_connect_commands() {
    let result = parse_socks5_target_from_client_bytes(&[
        0x05, 0x01, 0x00, // greeting: no auth
        0x05, 0x02, 0x00, 0x01, // BIND is unsupported
        127, 0, 0, 1, 0x00, 0x16,
    ])
    .await;

    assert_eq!(
        result.unwrap_err(),
        "only SOCKS5 CONNECT is supported".to_owned()
    );
}

#[tokio::test]
async fn socks5_target_rejects_unknown_address_types() {
    let result = parse_socks5_target_from_client_bytes(&[
        0x05, 0x01, 0x00, // greeting: no auth
        0x05, 0x01, 0x00, 0x7F, // unknown address type
        0x00, 0x16,
    ])
    .await;

    assert_eq!(
        result.unwrap_err(),
        "unsupported SOCKS address type: 127".to_owned()
    );
}

#[tokio::test]
async fn socks5_target_rejects_unsupported_version() {
    let error = parse_socks5_target_with_error(
        &[
            0x04, 0x01, 0x00, // unsupported version
            0x05, 0x01, 0x00, 0x01, 127, 0, 0, 1, 0x00, 0x16,
        ],
        false,
    )
    .await
    .unwrap();

    assert_eq!(error, "unsupported SOCKS version".to_owned());
}

#[tokio::test]
async fn socks5_target_rejects_invalid_domain_bytes() {
    let error = parse_socks5_target_with_error(
        &[
            0x05, 0x01, 0x00, // greeting: no auth
            0x05, 0x01, 0x00, 0x03, // CONNECT domain
            3, 0xFF, 0xFF, 0xFF, 0x00, 0x16,
        ],
        true,
    )
    .await
    .unwrap();

    assert!(error.contains("invalid"));
}

#[tokio::test]
async fn socks5_target_rejects_short_port_read() {
    let error = parse_socks5_target_with_error(
        &[
            0x05, 0x01, 0x00, // greeting: no auth
            0x05, 0x01, 0x00, 0x01, // CONNECT IPv4
            127, 0, 0, 1, 0x1F, // missing low byte of port
        ],
        true,
    )
    .await
    .unwrap();

    assert!(error.contains("early eof") || error.contains("unexpected end"));
}

#[tokio::test]
async fn socks5_success_response_uses_ipv4_unspecified_endpoint() {
    let (mut client, mut server) = tokio::io::duplex(16);
    let client_task = tokio::spawn(async move {
        let mut response = [0_u8; 10];
        client.read_exact(&mut response).await.unwrap();
        response
    });

    write_socks5_success(&mut server).await.unwrap();

    assert_eq!(
        client_task.await.unwrap(),
        [0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0]
    );
}

#[tokio::test]
async fn socks5_success_response_maps_write_failure() {
    let error = write_socks5_success(&mut FailingWriter).await.unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::Other);
}

struct FailingWriter;

impl tokio::io::AsyncWrite for FailingWriter {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        _buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        std::task::Poll::Ready(Err(std::io::Error::other("injected write failure")))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
}

#[test]
fn command_data_message_becomes_output_event() {
    let session_id = session_id();
    let mut events = Vec::new();
    let mut exit_code = None;

    let should_stop = collect_command_message(
        session_id,
        ChannelMsg::Data {
            data: CryptoVec::from_slice(b"hello\n"),
        },
        &mut events,
        &mut exit_code,
    )
    .expect("数据消息应该可以转换为输出事件");

    assert!(!should_stop);
    assert_eq!(
        events,
        vec![BackendEvent::Output {
            session_id,
            line: "hello\n".to_owned(),
        }]
    );
    assert_eq!(exit_code, None);
}

#[test]
fn command_exit_status_is_recorded_without_stopping_collection() {
    let session_id = session_id();
    let mut events = Vec::new();
    let mut exit_code = None;

    let should_stop = collect_command_message(
        session_id,
        ChannelMsg::ExitStatus { exit_status: 127 },
        &mut events,
        &mut exit_code,
    )
    .expect("退出状态应该可以记录");

    assert!(!should_stop);
    assert!(events.is_empty());
    assert_eq!(exit_code, Some(127));
}

#[test]
fn command_close_message_stops_collection() {
    let session_id = session_id();
    let mut events = Vec::new();
    let mut exit_code = None;

    let should_stop =
        collect_command_message(session_id, ChannelMsg::Close, &mut events, &mut exit_code)
            .expect("关闭消息应该可以处理");

    assert!(should_stop);
    assert!(events.is_empty());
}

#[test]
fn command_failure_message_reports_channel_error() {
    let session_id = session_id();
    let mut events = Vec::new();
    let mut exit_code = None;

    let error =
        collect_command_message(session_id, ChannelMsg::Failure, &mut events, &mut exit_code)
            .expect_err("服务端拒绝 channel 请求应该返回通道错误");

    assert!(matches!(
        error,
        BackendExecutionError::ChannelFailed {
            operation,
            reason,
        } if operation == "channel request" && reason.contains("server rejected")
    ));
}

#[test]
fn channel_request_messages_map_to_status_or_error() {
    assert_eq!(
        collect_channel_request_message("request shell", ChannelMsg::Success)
            .expect("成功消息应该被接受"),
        ChannelRequestStatus::Accepted
    );
    assert_eq!(
        collect_channel_request_message(
            "request shell",
            ChannelMsg::WindowAdjusted { new_size: 80 }
        )
        .expect("普通控制消息应该继续等待"),
        ChannelRequestStatus::Pending
    );

    let rejected = collect_channel_request_message("request shell", ChannelMsg::Failure)
        .expect_err("失败消息应该映射为通道错误");
    assert!(matches!(
        rejected,
        BackendExecutionError::ChannelFailed {
            operation,
            reason,
        } if operation == "request shell" && reason == "server rejected channel request"
    ));
}

#[test]
fn channel_request_close_and_end_map_to_channel_error() {
    let closed = collect_channel_request_message("request pty", ChannelMsg::Close)
        .expect_err("关闭消息应该映射为请求失败");
    assert!(matches!(
        closed,
        BackendExecutionError::ChannelFailed {
            operation,
            reason,
        } if operation == "request pty" && reason == "channel closed before request succeeded"
    ));

    let ended = channel_request_ended_error("request pty");
    assert!(matches!(
        ended,
        BackendExecutionError::ChannelFailed {
            operation,
            reason,
        } if operation == "request pty" && reason == "channel ended before request succeeded"
    ));
}

#[test]
fn shell_message_maps_output_and_exit_status() {
    let session_id = session_id();

    let output = shell_message_to_event(
        session_id,
        ChannelMsg::ExtendedData {
            data: CryptoVec::from_slice(b"stderr"),
            ext: 1,
        },
    );
    let exit = shell_message_to_event(session_id, ChannelMsg::ExitStatus { exit_status: 0 });

    assert_eq!(
        output,
        Some(BackendEvent::Output {
            session_id,
            line: "stderr".to_owned(),
        })
    );
    assert_eq!(
        exit,
        Some(BackendEvent::CommandExited {
            session_id,
            exit_code: Some(0),
        })
    );
}

#[test]
fn channel_lifecycle_events_preserve_payloads() {
    let session_id = session_id();

    assert_eq!(
        shell_opened_event(session_id),
        BackendEvent::ShellOpened { session_id }
    );
    assert_eq!(
        remote_command_started_event(session_id, "uptime".to_owned()),
        BackendEvent::RemoteCommandStarted {
            session_id,
            command: "uptime".to_owned(),
        }
    );
    assert_eq!(
        command_exited_event(session_id, Some(7)),
        BackendEvent::CommandExited {
            session_id,
            exit_code: Some(7),
        }
    );
    assert_eq!(
        disconnected_event(session_id),
        BackendEvent::Disconnected { session_id }
    );
}

#[test]
fn connection_lifecycle_events_preserve_payloads() {
    let session_id = session_id();
    let check = HostKeyCheck {
        host: "example.com".to_owned(),
        port: 2222,
        key_algorithm: KeyAlgorithm::Ed25519,
        verification: HostKeyVerification::Trusted,
        accepted: true,
        fingerprint: "SHA256:test".to_owned(),
    };

    assert_eq!(
        connecting_event(session_id, "example.com:2222".to_owned()),
        BackendEvent::Connecting {
            session_id,
            endpoint: "example.com:2222".to_owned(),
        }
    );
    assert_eq!(
        host_key_verified_event(session_id, check),
        BackendEvent::HostKeyVerified {
            session_id,
            host: "example.com".to_owned(),
            port: 2222,
            key_algorithm: KeyAlgorithm::Ed25519,
            fingerprint: "SHA256:test".to_owned(),
            result: HostKeyVerification::Trusted,
        }
    );
    assert_eq!(
        authenticating_event(session_id, "alice".to_owned()),
        BackendEvent::Authenticating {
            session_id,
            username: "alice".to_owned(),
        }
    );
    assert_eq!(
        authenticated_event(session_id),
        BackendEvent::Authenticated { session_id }
    );
    assert_eq!(
        connected_event(session_id),
        BackendEvent::Connected { session_id }
    );
}

#[test]
fn shell_failure_message_maps_to_failed_event() {
    let session_id = session_id();

    let event = shell_message_to_event(session_id, ChannelMsg::Failure);

    assert_eq!(
        event,
        Some(BackendEvent::Failed {
            session_id,
            reason: "server rejected channel request".to_owned(),
        })
    );
}

#[test]
fn shell_close_message_maps_to_disconnected() {
    let session_id = session_id();

    let event = shell_message_to_event(session_id, ChannelMsg::Close);

    assert_eq!(event, Some(BackendEvent::Disconnected { session_id }));
}

#[test]
fn command_exit_signal_message_maps_to_output_event() {
    let session_id = session_id();
    let signal_name = Sig::TERM;
    let expected_line = format!("远程进程收到信号退出：{signal_name:?}");
    let mut events = Vec::new();
    let mut exit_code = None;

    let should_stop = collect_command_message(
        session_id,
        ChannelMsg::ExitSignal {
            signal_name,
            core_dumped: false,
            error_message: String::new(),
            lang_tag: String::new(),
        },
        &mut events,
        &mut exit_code,
    )
    .expect("退出信号应该可以记录");

    assert!(!should_stop);
    assert_eq!(
        events,
        vec![BackendEvent::Output {
            session_id,
            line: expected_line,
        }]
    );
    assert_eq!(exit_code, None);
}

#[test]
fn shell_message_ignores_non_terminal_control_messages() {
    let session_id = session_id();

    assert_eq!(
        shell_message_to_event(session_id, ChannelMsg::WindowAdjusted { new_size: 80 }),
        None
    );
    assert_eq!(
        shell_message_to_event(session_id, ChannelMsg::Success),
        None
    );
}
