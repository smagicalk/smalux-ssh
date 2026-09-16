//! smalux-ssh 存储实现层。
//!
//! 实现 `smagical-core` 中定义的统一数据仓储接口 (`AppStorage`)，
//! 提供内存模拟存储 (`MockStorage`) 与后续的持久化存储引擎实现。

pub mod mock;
pub use mock::MockStorage;
