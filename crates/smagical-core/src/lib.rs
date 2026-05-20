//! 轻量核心领域模型。
//!
//! 本 crate 只承载不依赖 UI、终端、SSH 和本地存储的纯数据模型，方便主应用按功能边界拆分编译。

pub mod history;
pub mod host;
pub mod ids;
pub mod security;
pub mod session;
pub mod sftp;
pub mod snippet;
pub mod tunnel;
pub mod visual;
pub mod workspace;

pub use history::*;
pub use host::*;
pub use ids::*;
pub use security::*;
pub use session::*;
pub use sftp::*;
pub use snippet::*;
pub use tunnel::*;
pub use visual::*;
pub use workspace::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_module_reexports_core_domain_types() {
        let host_id = HostId(uuid::Uuid::new_v4());
        let history_id = CommandHistoryId(uuid::Uuid::new_v4());
        let item = CommandHistoryItem {
            id: history_id,
            host_id: Some(host_id),
            command: "pwd".to_owned(),
            working_directory: None,
            exit_code: Some(0),
            started_at_unix_secs: 1_700_000_000,
            duration_ms: Some(12),
        };

        assert_eq!(item.host_id, Some(host_id));
    }
}
