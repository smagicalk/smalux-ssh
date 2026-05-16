use super::sftp::{join_remote_path, parent_remote_dir, sftp_entry_from_parts, transfer_event};
use super::*;
use crate::model::{SftpEntryKind, TransferId, TransferStatus};
use russh::CryptoVec;
use russh_sftp::protocol::FileAttributes;
use uuid::Uuid;

fn session_id() -> SessionId {
    SessionId(Uuid::new_v4())
}

#[test]
fn pty_dimensions_are_never_zero() {
    let size = TerminalSize::new(0, 0);

    assert_eq!(columns(size), 1);
    assert_eq!(rows(size), 1);
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
