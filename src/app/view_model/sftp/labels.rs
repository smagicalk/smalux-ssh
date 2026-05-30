//! SFTP 展示标签。

use crate::model::SftpEntryKind;

use super::super::i18n::{Locale, tr};

pub(super) fn sftp_kind_label(kind: &SftpEntryKind, locale: Locale) -> &'static str {
    match kind {
        SftpEntryKind::File => tr(locale, "sftp.kind.file"),
        SftpEntryKind::Directory => tr(locale, "sftp.kind.directory"),
        SftpEntryKind::Symlink => tr(locale, "sftp.kind.symlink"),
        SftpEntryKind::Other => tr(locale, "sftp.kind.other"),
    }
}
