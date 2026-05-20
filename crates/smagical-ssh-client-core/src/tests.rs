use super::*;
use russh::client;
use std::time::Duration;

use russh::ChannelMsg;
use russh::CryptoVec;
use russh::Sig;
use russh::keys::PublicKey;
use russh_sftp::protocol::FileAttributes;
use smagical_backend_core::{BackendEvent, BackendExecutionError};
use smagical_core::{
    HostKeyVerification, KeyAlgorithm, KnownHostEntry, SftpEntryKind, TransferId, TransferStatus,
};
use uuid::Uuid;

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
