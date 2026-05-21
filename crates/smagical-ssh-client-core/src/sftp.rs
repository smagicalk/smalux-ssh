//! SFTP 纯映射 helper。

use russh_sftp::protocol::{FileAttributes, FileType as RusshSftpFileType};
use smagical_backend_core::{BackendEvent, BackendExecutionError};
use smagical_core::{SessionId, SftpEntry, SftpEntryKind, TransferId, TransferStatus};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const SFTP_TRANSFER_CHUNK_SIZE: usize = 64 * 1024;

/// SSH SFTP subsystem 名称。
pub const SFTP_SUBSYSTEM_NAME: &str = "sftp";

/// 打开 SFTP session channel 的操作名。
pub const OPEN_SFTP_SESSION_OPERATION: &str = "open sftp session";

/// 请求 SFTP subsystem 的操作名。
pub const REQUEST_SFTP_OPERATION: &str = "request sftp";

/// 初始化 SFTP 子系统的操作名。
pub const OPEN_SFTP_OPERATION: &str = "open";

/// 关闭 SFTP 子系统的操作名。
pub const CLOSE_SFTP_OPERATION: &str = "close";

/// SFTP 列目录操作名。
pub const LIST_DIR_OPERATION: &str = "list dir";

/// SFTP 删除文件操作名。
pub const REMOVE_FILE_OPERATION: &str = "remove file";

/// SFTP 创建目录操作名。
pub const CREATE_DIR_OPERATION: &str = "create dir";

/// SFTP 上传打开本地文件操作名。
pub const UPLOAD_OPEN_LOCAL_OPERATION: &str = "upload open local";

/// SFTP 上传读取本地文件操作名。
pub const UPLOAD_READ_LOCAL_OPERATION: &str = "upload read local";

/// SFTP 上传读取本地文件元数据操作名。
pub const UPLOAD_STAT_LOCAL_OPERATION: &str = "upload stat local";

/// SFTP 上传打开远程文件操作名。
pub const UPLOAD_OPEN_REMOTE_OPERATION: &str = "upload open remote";

/// SFTP 上传写入远程文件操作名。
pub const UPLOAD_WRITE_REMOTE_OPERATION: &str = "upload write remote";

/// SFTP 上传关闭远程文件操作名。
pub const UPLOAD_CLOSE_REMOTE_OPERATION: &str = "upload close remote";

/// SFTP 下载打开远程文件操作名。
pub const DOWNLOAD_OPEN_REMOTE_OPERATION: &str = "download open remote";

/// SFTP 下载读取远程文件操作名。
pub const DOWNLOAD_READ_REMOTE_OPERATION: &str = "download read remote";

/// SFTP 下载读取远程文件元数据操作名。
pub const DOWNLOAD_STAT_REMOTE_OPERATION: &str = "download stat remote";

/// SFTP 下载打开本地文件操作名。
pub const DOWNLOAD_OPEN_LOCAL_OPERATION: &str = "download open local";

/// SFTP 下载写入本地文件操作名。
pub const DOWNLOAD_WRITE_LOCAL_OPERATION: &str = "download write local";

/// SFTP 下载 flush 本地文件操作名。
pub const DOWNLOAD_FLUSH_LOCAL_OPERATION: &str = "download flush local";

/// SFTP 下载关闭远程文件操作名。
pub const DOWNLOAD_CLOSE_REMOTE_OPERATION: &str = "download close remote";

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

/// 创建 SFTP 目录列表事件。
pub fn sftp_entries_event(
    session_id: SessionId,
    remote_path: String,
    entries: Vec<SftpEntry>,
) -> BackendEvent {
    BackendEvent::SftpEntries {
        session_id,
        remote_path,
        entries,
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

/// 复制 SFTP 传输流并为每个数据块生成进度事件。
pub async fn copy_transfer_with_progress<R, W>(
    session_id: SessionId,
    transfer_id: TransferId,
    total_bytes: Option<u64>,
    reader: &mut R,
    writer: &mut W,
    read_operation: &str,
    write_operation: &str,
) -> Result<(u64, Vec<BackendEvent>), BackendExecutionError>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut transferred_bytes = 0_u64;
    let mut events = Vec::new();
    let mut buffer = vec![0_u8; SFTP_TRANSFER_CHUNK_SIZE];

    loop {
        let bytes_read = reader
            .read(&mut buffer)
            .await
            .map_err(|error| sftp_io_error(read_operation, error))?;

        if bytes_read == 0 {
            break;
        }

        writer
            .write_all(&buffer[..bytes_read])
            .await
            .map_err(|error| sftp_io_error(write_operation, error))?;
        transferred_bytes += bytes_read as u64;
        events.push(transfer_event(
            session_id,
            transfer_id,
            total_bytes,
            transferred_bytes,
            TransferStatus::Running,
        ));
    }

    Ok((transferred_bytes, events))
}

/// 将 SFTP 协议错误转换成后端执行错误。
pub fn sftp_error(operation: &str, error: impl std::fmt::Display) -> BackendExecutionError {
    BackendExecutionError::SftpFailed {
        operation: operation.to_owned(),
        reason: error.to_string(),
    }
}

/// 将 SFTP 本地 IO 错误转换成后端执行错误。
pub fn sftp_io_error(operation: &str, error: std::io::Error) -> BackendExecutionError {
    sftp_error(operation, error)
}

/// 判断执行错误是否来自 SFTP 子系统。
pub fn is_sftp_failure(error: &BackendExecutionError) -> bool {
    matches!(error, BackendExecutionError::SftpFailed { .. })
}

/// 返回 SFTP 错误中的操作名和原因。
pub fn sftp_failure_parts(error: &BackendExecutionError) -> Option<(&str, &str)> {
    match error {
        BackendExecutionError::SftpFailed { operation, reason } => Some((operation, reason)),
        _ => None,
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
