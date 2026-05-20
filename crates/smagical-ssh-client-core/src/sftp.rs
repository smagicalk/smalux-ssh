//! SFTP 纯映射 helper。

use russh_sftp::protocol::{FileAttributes, FileType as RusshSftpFileType};
use smagical_backend_core::BackendEvent;
use smagical_core::{SessionId, SftpEntry, SftpEntryKind, TransferId, TransferStatus};

/// 从 russh-sftp 目录项元数据创建领域 SFTP 目录项。
pub fn sftp_entry_from_parts(
    remote_dir: &str,
    name: String,
    metadata: FileAttributes,
) -> SftpEntry {
    SftpEntry {
        remote_path: join_remote_path(remote_dir, &name),
        name,
        kind: sftp_entry_kind(metadata.file_type()),
        size: metadata.size,
        modified_at_unix_secs: metadata.mtime.map(u64::from),
        permissions: metadata.permissions,
    }
}

/// 拼接远程目录和文件名。
pub fn join_remote_path(remote_dir: &str, name: &str) -> String {
    if remote_dir == "/" {
        format!("/{name}")
    } else {
        format!("{}/{}", remote_dir.trim_end_matches('/'), name)
    }
}

/// 返回远程路径的父目录。
pub fn parent_remote_dir(remote_path: &str) -> String {
    let path = remote_path.trim_end_matches('/');
    match path.rfind('/') {
        Some(0) | None => "/".to_owned(),
        Some(index) => path[..index].to_owned(),
    }
}

/// 创建 SFTP 传输进度事件。
pub fn transfer_event(
    session_id: SessionId,
    transfer_id: TransferId,
    total_bytes: Option<u64>,
    transferred_bytes: u64,
    status: TransferStatus,
) -> BackendEvent {
    BackendEvent::TransferProgress {
        session_id,
        transfer_id,
        total_bytes,
        transferred_bytes,
        status,
    }
}

fn sftp_entry_kind(file_type: RusshSftpFileType) -> SftpEntryKind {
    match file_type {
        RusshSftpFileType::Dir => SftpEntryKind::Directory,
        RusshSftpFileType::File => SftpEntryKind::File,
        RusshSftpFileType::Symlink => SftpEntryKind::Symlink,
        RusshSftpFileType::Other => SftpEntryKind::Other,
    }
}
