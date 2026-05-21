//! 本地安全资产的管理操作。
//!
//! 这里专门处理凭据元数据和 Known Hosts 的增删改，避免把存储管理逻辑继续塞进
//! `app_state.rs` 主文件。

#[path = "storage_admin/credential.rs"]
mod credential;
#[path = "storage_admin/known_hosts.rs"]
mod known_hosts;
#[cfg(test)]
#[path = "storage_admin/tests.rs"]
mod tests;
