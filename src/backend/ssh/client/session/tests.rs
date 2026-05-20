use super::sftp::{join_remote_path, parent_remote_dir, sftp_entry_from_parts, transfer_event};
use super::*;
use crate::model::{SftpEntryKind, TransferId, TransferStatus};
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
