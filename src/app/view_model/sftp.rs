//! SFTP 面板展示模型。

mod browser;
mod labels;
mod types;

pub(super) use browser::active_sftp;
pub(in crate::app) use types::{SftpEntryViewModel, SftpViewModel};
