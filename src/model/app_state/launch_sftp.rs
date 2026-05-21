//! SFTP 浏览器、书签和可用会话调度入口。

#[path = "sftp_browser/bookmarks.rs"]
mod bookmarks;
#[path = "sftp_browser/open.rs"]
mod open;
#[path = "sftp_browser/session.rs"]
pub(super) mod session;
