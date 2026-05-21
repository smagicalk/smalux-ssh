//! SFTP 传输和远端操作的路径 helper。

use std::path::Path;

pub(super) fn basename_local_path(path: &str) -> Option<String> {
    Path::new(path)
        .file_name()
        .and_then(|file_name| file_name.to_str())
        .map(ToOwned::to_owned)
}

pub(super) fn is_plain_remote_name(name: &str) -> bool {
    !matches!(name, "." | "..") && !name.contains('/') && !name.contains('\\')
}
