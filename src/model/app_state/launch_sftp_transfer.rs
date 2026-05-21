//! SFTP 上传、下载、取消和远端路径操作调度入口。

#[path = "sftp_transfer/cancel.rs"]
mod cancel;
#[path = "sftp_transfer/download.rs"]
mod download;
#[path = "sftp_transfer/path.rs"]
mod path;
#[path = "sftp_transfer/remote_actions.rs"]
mod remote_actions;
#[path = "sftp_transfer/upload.rs"]
mod upload;
