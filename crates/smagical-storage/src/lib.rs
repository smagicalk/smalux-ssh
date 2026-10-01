//! smalux-ssh 存储实现层。
//!
//! 实现 `smagical-core` 中定义的统一数据仓储接口 (`AppStorage`)，
//! 提供内存模拟存储 (`MockStorage`) 与后续的持久化存储引擎实现。

pub mod crypto;
pub mod entities;
pub mod mock;
pub mod seaorm;
pub mod storage_mode;

pub use mock::MockStorage;
pub use seaorm::SeaOrmStorage;
pub use storage_mode::{get_persisted_storage_mode, get_storage_config_path, save_persisted_storage_mode};
