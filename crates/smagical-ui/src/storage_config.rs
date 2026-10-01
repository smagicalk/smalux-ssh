//! 存储后端模式本地持久化配置模块 (Storage Mode Config)。
//!
//! 核心配置操作已下沉至 `smagical_storage::storage_mode`，此处保留别名重导出以确保向下兼容与零断裂。

pub use smagical_storage::storage_mode::*;
