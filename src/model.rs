//! 领域模型聚合和应用根状态。
//!
//! 具体领域类型按单一职责拆分到 `src/model/` 子模块中，本文件只负责对外导出稳定 API，
//! 并保留 Iced 应用根状态，避免单文件继续膨胀。

mod app_state;
mod ui_state;

pub use app_state::*;
pub use ui_state::*;

pub use smagical_core::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_module_reexports_domain_types() {
        let host_id = HostId(uuid::Uuid::new_v4());
        let tab = SessionTab {
            id: SessionId(uuid::Uuid::new_v4()),
            host_id: Some(host_id),
            kind: SessionKind::Shell,
            title: "shell".to_owned(),
            status: SessionStatus::Connecting,
        };

        assert_eq!(tab.host_id, Some(host_id));
    }
}
