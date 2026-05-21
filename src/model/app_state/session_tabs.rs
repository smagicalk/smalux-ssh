//! 会话标签页生命周期处理。
//!
//! 负责关闭、激活会话标签页，以及清理关联的终端、SFTP 和隧道运行态。

#[path = "session_tabs/activate.rs"]
mod activate;
#[path = "session_tabs/close.rs"]
mod close;
#[path = "session_tabs/pending.rs"]
mod pending;
#[path = "session_tabs/sftp_cleanup.rs"]
mod sftp_cleanup;
#[path = "session_tabs/tunnel_cleanup.rs"]
mod tunnel_cleanup;
