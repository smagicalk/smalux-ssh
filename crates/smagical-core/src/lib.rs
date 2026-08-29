//! smagicalssh 核心 crate。
//!
//! 这里不依赖具体 UI 框架，只负责领域模型、核心状态和服务接口。

#![deny(missing_docs)]

pub mod domain;
pub mod state;
pub mod storage;
pub mod theme;

pub use domain::{group::GroupRecord, host::HostRecord};
pub use state::core_state::CoreState;
pub use storage::{
    AppStorage, GroupRepository, HostRepository, MockStorage, StorageError, StorageResult,
};
