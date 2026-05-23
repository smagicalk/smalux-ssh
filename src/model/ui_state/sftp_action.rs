//! SFTP 操作草稿。

#[path = "sftp_action/types.rs"]
mod types;
#[path = "sftp_action/ui.rs"]
mod ui;

pub use types::{SftpActionDraft, SftpActionDraftField};

#[cfg(test)]
#[path = "sftp_action/tests.rs"]
mod tests;
