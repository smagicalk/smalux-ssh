//! SFTP 传输和远端操作的路径 helper。

pub(super) fn is_plain_remote_name(name: &str) -> bool {
    !matches!(name, "." | "..") && !name.contains('/') && !name.contains('\\')
}
