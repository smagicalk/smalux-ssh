//! smagicalssh 核心 crate。
//!
//! 这里不依赖具体 UI 框架，只负责领域模型、核心状态和服务接口。

pub mod backend;
pub mod config;
pub mod domain;
pub mod services;
pub mod session;
pub mod state;
pub mod storage;
pub mod terminal;
pub mod theme;

pub use domain::host::HostRecord;
pub use state::core_state::CoreState;
