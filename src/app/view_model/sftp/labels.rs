//! SFTP 展示标签。

use crate::model::SftpEntryKind;

pub(super) fn sftp_kind_label(kind: &SftpEntryKind) -> &'static str {
    match kind {
        SftpEntryKind::File => "File",
        SftpEntryKind::Directory => "Dir",
        SftpEntryKind::Symlink => "Link",
        SftpEntryKind::Other => "Other",
    }
}
